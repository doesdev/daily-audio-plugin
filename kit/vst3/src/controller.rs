//! The host-facing side: parameter names, text and values straight
//! from the spec, and on Windows the web editor.

use crate::processor::read_all;
use crate::strings::{copy_wstring, read_wstring};
use kit::{Spec, state};
use kit_webui::feed::ScopeLink;
use std::cell::{Cell, RefCell};
use std::ffi::c_char;
use std::rc::Rc;
use vst3::Steinberg::Vst::*;
use vst3::Steinberg::*;
use vst3::{Class, ComPtr, ComRef};

// Settings, then meters; shared with the editor on the UI thread.
pub type Values = Rc<[Cell<f64>]>;
pub type Handler = Rc<RefCell<Option<ComPtr<IComponentHandler>>>>;

/// The first of 128 parameters that stand in for the MIDI controllers.
/// VST3 gives a plugin no controller messages: the host turns one into
/// a change of whichever parameter `IMidiMapping` names for it. These
/// are hidden and cannot be automated; the processor reads them as the
/// controllers they are.
pub const CC_PARAMS: u32 = 0x4D43_0000;

/// The controller a parameter id stands in for.
pub fn cc_of(id: u32) -> Option<u8> {
    id.checked_sub(CC_PARAMS)
        .and_then(|cc| u8::try_from(cc).ok())
        .filter(|&cc| cc < 128)
}

pub struct Controller {
    spec: Spec,
    values: Values,
    handler: Handler,
    // The sound, once the host has connected the processor.
    pub(crate) scope: ScopeLink,
    // Where each stand-in for a controller was last put: a host may
    // ask for it back.
    ccs: Rc<[Cell<f64>]>,
}

impl Class for Controller {
    type Interfaces = (IEditController, IConnectionPoint, IMidiMapping);
}

impl IMidiMappingTrait for Controller {
    unsafe fn getMidiControllerAssignment(
        &self,
        bus: i32,
        _channel: i16,
        controller: CtrlNumber,
        id: *mut ParamID,
    ) -> tresult {
        let cc = u32::try_from(controller).ok().filter(|&cc| cc < 128);
        let (0, Some(cc), Some(id)) = (bus, cc, unsafe { id.as_mut() }) else {
            return kResultFalse;
        };
        *id = CC_PARAMS + cc;
        kResultTrue
    }
}

// The processor offers its scope when the host connects the two; all
// the controller does is take it.
impl IConnectionPointTrait for Controller {
    unsafe fn connect(&self, _other: *mut IConnectionPoint) -> tresult {
        kResultOk
    }

    unsafe fn disconnect(&self, _other: *mut IConnectionPoint) -> tresult {
        kResultOk
    }

    unsafe fn notify(&self, message: *mut IMessage) -> tresult {
        match crate::connect::accept(message) {
            Some(scope) => {
                *self.scope.borrow_mut() = Some(scope);
                kResultOk
            }
            None => kResultFalse,
        }
    }
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
            scope: Rc::default(),
            ccs: (0..128).map(|_| Cell::new(0.0)).collect(),
        }
    }

    fn cell(&self, id: u32) -> Option<&Cell<f64>> {
        match cc_of(id) {
            Some(cc) => self.ccs.get(usize::from(cc)),
            None => self.spec.index_of(id).and_then(|i| self.values.get(i)),
        }
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
        self.spec.total() as i32 + 128
    }

    unsafe fn getParameterInfo(&self, index: i32, info: *mut ParameterInfo) -> tresult {
        let at = usize::try_from(index).ok();
        let Some(info) = (unsafe { info.as_mut() }) else {
            return kInvalidArgument;
        };
        // After the plugin's own: the stand-ins for the controllers.
        let cc = at
            .and_then(|i| i.checked_sub(self.spec.total()))
            .filter(|&cc| cc < 128);
        if let Some(cc) = cc {
            info.id = CC_PARAMS + cc as u32;
            copy_wstring(&format!("MIDI CC {cc}"), &mut info.title);
            copy_wstring(&format!("CC {cc}"), &mut info.shortTitle);
            copy_wstring("", &mut info.units);
            info.stepCount = 0;
            info.defaultNormalizedValue = 0.0;
            info.unitId = 0;
            info.flags = ParameterInfo_::ParameterFlags_::kIsHidden as i32;
            return kResultOk;
        }
        let Some(p) = at.and_then(|i| self.spec.row(i)) else {
            return kInvalidArgument;
        };

        info.id = p.id;
        copy_wstring(p.title, &mut info.title);
        copy_wstring(p.short, &mut info.shortTitle);
        copy_wstring("", &mut info.units);
        info.stepCount = p.steps();
        info.defaultNormalizedValue = p.default_norm();
        info.unitId = 0;
        use ParameterInfo_::ParameterFlags_::{kCanAutomate, kIsBypass, kIsList, kIsReadOnly};
        info.flags = match (p.read_only(), p.is_list()) {
            (true, _) => kIsReadOnly as i32,
            // The host's own bypass button and the plugin's are then one.
            (false, true) if p.is_bypass() => (kCanAutomate | kIsList | kIsBypass) as i32,
            (false, true) => (kCanAutomate | kIsList) as i32,
            (false, false) => kCanAutomate as i32,
        };
        kResultOk
    }

    unsafe fn getParamStringByValue(&self, id: u32, norm: f64, string: *mut String128) -> tresult {
        let Some(out) = (unsafe { string.as_mut() }) else {
            return kInvalidArgument;
        };
        if cc_of(id).is_some() {
            copy_wstring(&format!("{:.0}", norm.clamp(0.0, 1.0) * 127.0), out);
            return kResultOk;
        }
        let Some(p) = self.spec.by_id(id) else {
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
        match cc_of(id) {
            Some(_) => norm,
            None => self.spec.by_id(id).map_or(0.0, |p| p.plain(norm)),
        }
    }

    unsafe fn plainParamToNormalized(&self, id: u32, plain: f64) -> f64 {
        match cc_of(id) {
            Some(_) => plain,
            None => self.spec.by_id(id).map_or(0.0, |p| p.normalized(plain)),
        }
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

        crate::editor::Editor::create(
            self.spec,
            self.values.clone(),
            self.handler.clone(),
            self.scope.clone(),
        )
    }

    #[cfg(not(windows))]
    unsafe fn createView(&self, _name: *const c_char) -> *mut IPlugView {
        std::ptr::null_mut()
    }
}
