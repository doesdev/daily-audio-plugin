//! The host-facing side: parameter names, text and values straight
//! from the spec, and on Windows the web editor.

use crate::processor::read_all;
use crate::strings::{copy_wstring, read_wstring};
use kit::{Spec, state};
use std::cell::{Cell, RefCell};
use std::ffi::c_char;
use std::rc::Rc;
use vst3::Steinberg::Vst::*;
use vst3::Steinberg::*;
use vst3::{Class, ComPtr, ComRef};

// Settings, then meters; shared with the editor on the UI thread.
pub type Values = Rc<[Cell<f64>]>;
pub type Handler = Rc<RefCell<Option<ComPtr<IComponentHandler>>>>;

pub struct Controller {
    spec: Spec,
    values: Values,
    handler: Handler,
}

impl Class for Controller {
    type Interfaces = (IEditController,);
}

impl Controller {
    pub fn new(spec: Spec) -> Self {
        let values = spec
            .rows()
            .map(|p| Cell::new(p.default_norm()))
            .collect::<Vec<_>>();
        Self {
            spec,
            values: values.into(),
            handler: Rc::default(),
        }
    }

    fn cell(&self, id: u32) -> Option<&Cell<f64>> {
        self.spec.index_of(id).and_then(|i| self.values.get(i))
    }
}

impl IPluginBaseTrait for Controller {
    unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }

    unsafe fn terminate(&self) -> tresult {
        self.handler.borrow_mut().take();
        kResultOk
    }
}

impl IEditControllerTrait for Controller {
    // The processor saved the values; mirror them here.
    unsafe fn setComponentState(&self, stream: *mut IBStream) -> tresult {
        let Some(stream) = (unsafe { ComRef::from_raw(stream) }) else {
            return kInvalidArgument;
        };

        let bytes = match unsafe { read_all(&stream) } {
            Ok(bytes) => bytes,
            Err(rc) => return rc,
        };
        let Some(values) = state::decode(&self.spec, &bytes) else {
            return kResultFalse;
        };
        self.values
            .iter()
            .zip(values)
            .for_each(|(cell, v)| cell.set(v));
        kResultOk
    }

    unsafe fn setState(&self, _state: *mut IBStream) -> tresult {
        kResultOk
    }

    unsafe fn getState(&self, _state: *mut IBStream) -> tresult {
        kResultOk
    }

    unsafe fn getParameterCount(&self) -> i32 {
        self.spec.total() as i32
    }

    unsafe fn getParameterInfo(&self, index: i32, info: *mut ParameterInfo) -> tresult {
        let row = usize::try_from(index).ok().and_then(|i| self.spec.row(i));
        let (Some(p), Some(info)) = (row, unsafe { info.as_mut() }) else {
            return kInvalidArgument;
        };

        info.id = p.id;
        copy_wstring(p.title, &mut info.title);
        copy_wstring(p.short, &mut info.shortTitle);
        copy_wstring("", &mut info.units);
        info.stepCount = p.steps();
        info.defaultNormalizedValue = p.default_norm();
        info.unitId = 0;
        use ParameterInfo_::ParameterFlags_::{kCanAutomate, kIsList, kIsReadOnly};
        info.flags = match (p.read_only(), p.is_list()) {
            (true, _) => kIsReadOnly as i32,
            (false, true) => (kCanAutomate | kIsList) as i32,
            (false, false) => kCanAutomate as i32,
        };
        kResultOk
    }

    unsafe fn getParamStringByValue(&self, id: u32, norm: f64, string: *mut String128) -> tresult {
        let (Some(p), Some(out)) = (self.spec.by_id(id), unsafe { string.as_mut() }) else {
            return kInvalidArgument;
        };

        copy_wstring(&p.display(norm), out);
        kResultOk
    }

    unsafe fn getParamValueByString(&self, id: u32, string: *mut TChar, norm: *mut f64) -> tresult {
        let Some(p) = self.spec.by_id(id) else {
            return kInvalidArgument;
        };
        let parsed = unsafe { read_wstring(string) }.and_then(|text| p.parse(&text));
        let (Some(value), Some(norm)) = (parsed, unsafe { norm.as_mut() }) else {
            return kResultFalse;
        };

        *norm = value;
        kResultOk
    }

    unsafe fn normalizedParamToPlain(&self, id: u32, norm: f64) -> f64 {
        self.spec.by_id(id).map_or(0.0, |p| p.plain(norm))
    }

    unsafe fn plainParamToNormalized(&self, id: u32, plain: f64) -> f64 {
        self.spec.by_id(id).map_or(0.0, |p| p.normalized(plain))
    }

    unsafe fn getParamNormalized(&self, id: u32) -> f64 {
        self.cell(id).map_or(0.0, Cell::get)
    }

    unsafe fn setParamNormalized(&self, id: u32, value: f64) -> tresult {
        let Some(cell) = self.cell(id) else {
            return kInvalidArgument;
        };

        cell.set(value.clamp(0.0, 1.0));
        kResultOk
    }

    unsafe fn setComponentHandler(&self, handler: *mut IComponentHandler) -> tresult {
        *self.handler.borrow_mut() = unsafe { ComRef::from_raw(handler) }.map(|h| h.to_com_ptr());
        kResultOk
    }

    #[cfg(windows)]
    unsafe fn createView(&self, name: *const c_char) -> *mut IPlugView {
        let editor =
            !name.is_null() && unsafe { std::ffi::CStr::from_ptr(name) }.to_bytes() == b"editor";
        if !editor {
            return std::ptr::null_mut();
        }

        crate::editor::Editor::create(self.spec, self.values.clone(), self.handler.clone())
    }

    #[cfg(not(windows))]
    unsafe fn createView(&self, _name: *const c_char) -> *mut IPlugView {
        std::ptr::null_mut()
    }
}
