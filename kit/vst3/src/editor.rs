//! The plugin editor: the page in a child of the window the host hands
//! over. The page's actions become host automation gestures; nothing
//! here touches the audio thread.

use crate::controller::{Handler, Values};
use kit::Spec;
use kit_webui::feed::ScopeLink;
use kit_webui::{Action, OnAction, Panel};
use std::cell::{Cell, RefCell};
use std::ffi::{CStr, c_void};
use std::rc::Rc;
use vst3::Steinberg::Vst::*;
use vst3::Steinberg::*;
use vst3::{Class, ComPtr, ComRef, ComWrapper};

pub struct Editor {
    spec: Spec,
    values: Values,
    handler: Handler,
    panel: RefCell<Option<Panel>>,
    // Where the sound will be, once the processor has offered it.
    scope: RefCell<Option<ScopeLink>>,
    // In physical pixels: the spec's logical size times the scale.
    size: Cell<(i32, i32)>,
    scale: Cell<f32>,
    frame: RefCell<Option<ComPtr<IPlugFrame>>>,
    // This view's own interface pointer, for `resizeView`. Not counted:
    // the object owns itself.
    me: Cell<*mut IPlugView>,
}

impl Class for Editor {
    type Interfaces = (IPlugView, IPlugViewContentScaleSupport);
}

// Page actions as host automation gestures, the value kept as well.
fn gestures(spec: Spec, values: Values, handler: Handler) -> OnAction {
    Rc::new(move |action| {
        let held = handler.borrow();
        let Some(host) = held.as_ref() else {
            return;
        };
        let id = |index: usize| spec.params.get(index).map(|p| p.id);

        match action {
            Action::Begin(index) => {
                if let Some(id) = id(index) {
                    unsafe { host.beginEdit(id) };
                }
            }
            Action::End(index) => {
                if let Some(id) = id(index) {
                    unsafe { host.endEdit(id) };
                }
            }
            Action::Set(index, value) => {
                if let (Some(id), Some(cell)) = (id(index), values.get(index)) {
                    cell.set(value);
                    unsafe { host.performEdit(id, value) };
                }
            }
            Action::Ready | Action::Blur => {}
        }
    })
}

impl Editor {
    /// A new editor as the raw view pointer `createView` hands the host.
    pub fn create(
        spec: Spec,
        values: Values,
        handler: Handler,
        scope: ScopeLink,
    ) -> *mut IPlugView {
        let Some((editor, view)) = Self::wrap(spec, values, handler) else {
            return std::ptr::null_mut();
        };
        *editor.scope.borrow_mut() = Some(scope);

        view.into_raw()
    }

    pub(crate) fn wrap(
        spec: Spec,
        values: Values,
        handler: Handler,
    ) -> Option<(ComWrapper<Self>, ComPtr<IPlugView>)> {
        let (w, h) = spec.info.size;
        let editor = ComWrapper::new(Self {
            spec,
            values,
            handler,
            panel: RefCell::new(None),
            scope: RefCell::new(None),
            size: Cell::new((w as i32, h as i32)),
            scale: Cell::new(1.0),
            frame: RefCell::new(None),
            me: Cell::new(std::ptr::null_mut()),
        });
        let view = editor.to_com_ptr::<IPlugView>()?;

        editor.me.set(view.as_ptr());
        Some((editor, view))
    }

    fn open(&self, parent: *mut c_void) -> Option<()> {
        let on_action = gestures(self.spec, self.values.clone(), self.handler.clone());
        let panel = Panel::open(
            self.spec,
            parent,
            self.size.get(),
            self.values.clone(),
            on_action,
            Box::new(|| {}),
        )?;
        // The sound itself, from the processor once the host has
        // connected it; until then the feed has nothing to say.
        if let Some(scope) = self.scope.borrow().clone() {
            panel.feed(kit_webui::feed::live(self.spec, scope.clone()));
            panel.hear(kit_webui::feed::learner(self.spec, scope));
        }

        *self.panel.borrow_mut() = Some(panel);
        Some(())
    }

    fn close(&self) {
        self.panel.borrow_mut().take();
    }

    /// Runs `js` in the open page, for tests.
    #[cfg(test)]
    pub fn eval(&self, js: &str, done: impl Fn(String) + Send + 'static) -> bool {
        self.panel
            .borrow()
            .as_ref()
            .is_some_and(|panel| panel.eval(js, done))
    }
}

fn is_hwnd(kind: FIDString) -> bool {
    !kind.is_null() && unsafe { CStr::from_ptr(kind) }.to_bytes() == b"HWND"
}

impl IPlugViewTrait for Editor {
    unsafe fn isPlatformTypeSupported(&self, kind: FIDString) -> tresult {
        if is_hwnd(kind) {
            kResultTrue
        } else {
            kResultFalse
        }
    }

    unsafe fn attached(&self, parent: *mut c_void, kind: FIDString) -> tresult {
        if parent.is_null() || !is_hwnd(kind) {
            return kInvalidArgument;
        }

        match self.open(parent) {
            Some(()) => kResultOk,
            None => kResultFalse,
        }
    }

    unsafe fn removed(&self) -> tresult {
        self.close();
        kResultOk
    }

    // Keys and wheel the page does not claim go back to the host.
    unsafe fn onWheel(&self, _distance: f32) -> tresult {
        kResultFalse
    }

    unsafe fn onKeyDown(&self, _key: char16, _code: int16, _mods: int16) -> tresult {
        kResultFalse
    }

    unsafe fn onKeyUp(&self, _key: char16, _code: int16, _mods: int16) -> tresult {
        kResultFalse
    }

    unsafe fn getSize(&self, size: *mut ViewRect) -> tresult {
        let Some(size) = (unsafe { size.as_mut() }) else {
            return kInvalidArgument;
        };

        let (w, h) = self.size.get();
        *size = ViewRect {
            left: 0,
            top: 0,
            right: w,
            bottom: h,
        };
        kResultOk
    }

    unsafe fn onSize(&self, size: *mut ViewRect) -> tresult {
        let Some(size) = (unsafe { size.as_ref() }) else {
            return kInvalidArgument;
        };

        self.size
            .set((size.right - size.left, size.bottom - size.top));
        if let Some(panel) = self.panel.borrow().as_ref() {
            panel.resize(self.size.get());
        }
        kResultOk
    }

    unsafe fn onFocus(&self, _state: TBool) -> tresult {
        kResultOk
    }

    unsafe fn setFrame(&self, frame: *mut IPlugFrame) -> tresult {
        *self.frame.borrow_mut() = unsafe { ComRef::from_raw(frame) }.map(|f| f.to_com_ptr());
        kResultOk
    }

    unsafe fn canResize(&self) -> tresult {
        kResultTrue
    }

    unsafe fn checkSizeConstraint(&self, rect: *mut ViewRect) -> tresult {
        let Some(rect) = (unsafe { rect.as_mut() }) else {
            return kInvalidArgument;
        };

        let scale = self.scale.get();
        let (min_w, min_h) = self.spec.info.min_size;
        let min_w = (min_w as f32 * scale) as i32;
        let min_h = (min_h as f32 * scale) as i32;
        rect.right = rect.right.max(rect.left + min_w);
        rect.bottom = rect.bottom.max(rect.top + min_h);
        kResultTrue
    }
}

impl IPlugViewContentScaleSupportTrait for Editor {
    // The host says how big a logical pixel is; ask to be resized to
    // match, keeping the same logical size.
    unsafe fn setContentScaleFactor(
        &self,
        factor: IPlugViewContentScaleSupport_::ScaleFactor,
    ) -> tresult {
        if !(factor.is_finite() && factor > 0.0) {
            return kInvalidArgument;
        }

        let (w, h) = self.size.get();
        let old = self.scale.replace(factor);
        let resize = |v: i32| (v as f32 / old * factor).round() as i32;
        let mut rect = ViewRect {
            left: 0,
            top: 0,
            right: resize(w),
            bottom: resize(h),
        };
        self.size.set((rect.right, rect.bottom));

        let open = self.panel.borrow().is_some();
        let frame = self.frame.borrow();
        match frame.as_ref() {
            Some(frame) if open && !self.me.get().is_null() => unsafe {
                frame.resizeView(self.me.get(), &mut rect);
            },
            _ => {}
        }
        kResultOk
    }
}

// The editor in a real webview on a hidden window of this thread, the
// host played by a recorder: what a DAW does, minus the DAW.
#[cfg(test)]
mod tests {
    use super::*;
    use kit::{Info, Role, meter, range};
    use kit_webui::testing::{HostWindow, pump};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    const GAIN: u32 = 10;
    const LEVEL: u32 = 12;
    const INFO: Info = Info {
        name: "Editor test",
        vendor: "kit",
        url: "",
        description: "",
        version: "0.0.0",
        clap_id: "dev.kit.editor-test",
        vst3_processor: [1, 2, 3, 4],
        vst3_controller: [5, 6, 7, 8],
        role: Role::Effect,
        audio_in: 2,
        audio_out: 2,
        midi_in: false,
        midi_out: false,
        size: (480, 300),
        min_size: (320, 200),
    };
    const SPEC: Spec = Spec {
        info: &INFO,
        params: &[range(GAIN, "gain", "Gain", "Level", (0.0, 2.0, 1.0), "")],
        meters: &[meter(LEVEL, "level", "Level")],
        assets: &[(
            "/main.js",
            "text/javascript",
            "import { mount, auto } from './kit/kit.js'\nmount(auto)\n",
        )],
    };

    type Edits = Rc<RefCell<Vec<(char, u32, f64)>>>;

    struct Recorder(Edits);

    impl Class for Recorder {
        type Interfaces = (IComponentHandler,);
    }

    impl IComponentHandlerTrait for Recorder {
        unsafe fn beginEdit(&self, id: ParamID) -> tresult {
            self.0.borrow_mut().push(('b', id, 0.0));
            kResultOk
        }

        unsafe fn performEdit(&self, id: ParamID, value: ParamValue) -> tresult {
            self.0.borrow_mut().push(('p', id, value));
            kResultOk
        }

        unsafe fn endEdit(&self, id: ParamID) -> tresult {
            self.0.borrow_mut().push(('e', id, 0.0));
            kResultOk
        }

        unsafe fn restartComponent(&self, _flags: i32) -> tresult {
            kResultOk
        }
    }

    // The page's answer to `js`, JSON encoded.
    fn eval(editor: &Editor, js: &str) -> Option<String> {
        let out = Arc::new(Mutex::new(None));
        let slot = out.clone();
        if !editor.eval(js, move |r| *slot.lock().unwrap() = Some(r)) {
            return None;
        }

        let until = Instant::now() + Duration::from_secs(5);
        while Instant::now() < until {
            pump(10);
            if let Some(r) = out.lock().unwrap().take() {
                return Some(r);
            }
        }
        None
    }

    fn wait_for(editor: &Editor, js: &str, want: &str) {
        let until = Instant::now() + Duration::from_secs(20);
        let mut got = None;
        while Instant::now() < until {
            got = eval(editor, js);
            if got.as_deref() == Some(want) {
                return;
            }
            pump(20);
        }
        panic!("{js}: want {want}, got {got:?}");
    }

    fn fresh() -> (Values, Edits, Handler) {
        let values: Values = vec![Cell::new(0.5), Cell::new(0.0)].into();
        let edits: Edits = Rc::default();
        let recorder = ComWrapper::new(Recorder(edits.clone()));
        let handler = Rc::new(RefCell::new(recorder.to_com_ptr::<IComponentHandler>()));
        (values, edits, handler)
    }

    const READY: &str = "document.getElementById('app').dataset.ready";
    const GAIN_TEXT: &str = "document.querySelector('[data-key=\"gain\"] .kit-value').textContent";
    const LEVEL_TEXT: &str =
        "document.querySelector('[data-key=\"level\"] .kit-value').textContent";

    #[test]
    #[ignore = "opens a WebView2; npm run check runs it on Windows"]
    fn editor_round_trips_with_the_host() {
        let parent = HostWindow::open();
        let (values, edits, handler) = fresh();
        let (editor, view) = Editor::wrap(SPEC, values.clone(), handler).expect("editor");

        assert_eq!(
            unsafe { view.attached(parent.hwnd(), c"HWND".as_ptr()) },
            kResultOk
        );
        wait_for(&editor, READY, "\"true\"");
        wait_for(&editor, GAIN_TEXT, "\"1.00\"");

        // A drag on Gain in the page is one automation gesture.
        let gesture = format!(
            "for (const m of [{{type:'begin',id:{GAIN}}},{{type:'set',id:{GAIN},value:0.25}},{{type:'end',id:{GAIN}}}]) window.ipc.postMessage(JSON.stringify(m))"
        );
        eval(&editor, &gesture).expect("gesture");
        pump(100);
        let want = vec![('b', GAIN, 0.0), ('p', GAIN, 0.25), ('e', GAIN, 0.0)];
        assert_eq!(*edits.borrow(), want, "host saw the gesture");
        assert_eq!(values[0].get(), 0.25, "controller value");

        // Automation moves it back; the page follows, with the host's text.
        values[0].set(0.75);
        wait_for(&editor, GAIN_TEXT, "\"1.50\"");
        values[1].set(0.5);
        wait_for(&editor, LEVEL_TEXT, "\"-30 dB\"");

        // Nothing the page sends may move a meter or an unknown id.
        let bad = format!(
            "window.ipc.postMessage(JSON.stringify({{type:'set',id:{LEVEL},value:1}})); window.ipc.postMessage('junk')"
        );
        eval(&editor, &bad).expect("bad messages");
        pump(100);
        assert_eq!(edits.borrow().len(), 3, "ignored");

        assert_eq!(unsafe { view.removed() }, kResultOk);
    }

    #[test]
    #[ignore = "opens WebView2s; npm run check runs it on Windows"]
    fn editors_survive_open_close_and_reopen() {
        let parent = HostWindow::open();
        let editors: Vec<_> = (0..3)
            .map(|_| {
                let (values, _, handler) = fresh();
                Editor::wrap(SPEC, values, handler).expect("editor")
            })
            .collect();

        for _ in 0..4 {
            for (editor, view) in &editors {
                assert_eq!(
                    unsafe { view.attached(parent.hwnd(), c"HWND".as_ptr()) },
                    kResultOk
                );
                wait_for(editor, READY, "\"true\"");
            }
            for (_, view) in &editors {
                assert_eq!(unsafe { view.removed() }, kResultOk);
            }
            pump(200);
        }
    }
}
