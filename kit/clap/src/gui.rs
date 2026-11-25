//! The editor: the page embedded in the host's window (win32 only, not
//! floating). Edits become gestures the host hears at the next flush;
//! every tick it shows what the host and the audio thread did.

use crate::instance::Core;
use clap_sys::ext::gui::*;
use clap_sys::plugin::clap_plugin;
use kit::{BEGIN, END, SET};
use kit_webui::{Action, OnAction, Panel, Values};
use std::cell::Cell;
use std::ffi::{CStr, c_char, c_void};
use std::rc::Rc;

pub struct Gui {
    panel: Option<Panel>,
    // In physical pixels: the spec's logical size times the scale.
    size: (u32, u32),
    scale: f64,
}

impl Gui {
    /// Runs `js` in the open page, for tests.
    #[cfg(test)]
    pub fn eval(&self, js: &str, done: impl Fn(String) + Send + 'static) -> bool {
        self.panel
            .as_ref()
            .is_some_and(|panel| panel.eval(js, done))
    }
}

fn win32(api: *const c_char, floating: bool) -> bool {
    !floating && !api.is_null() && unsafe { CStr::from_ptr(api) } == CLAP_WINDOW_API_WIN32
}

// Gestures for the host, which hears them at its next flush.
fn gestures(core: *const Core) -> OnAction {
    Rc::new(move |action| {
        let Some(core) = (unsafe { core.as_ref() }) else {
            return;
        };

        let shared = &core.shared;
        match action {
            Action::Begin(index) => shared.edit(index, BEGIN),
            Action::Set(index, value) => {
                shared.set(index, value);
                shared.edit(index, SET);
            }
            Action::End(index) => shared.edit(index, END),
            Action::Ready | Action::Blur => return,
        }
        core.request_flush();
    })
}

unsafe extern "C" fn is_api_supported(
    _: *const clap_plugin,
    api: *const c_char,
    floating: bool,
) -> bool {
    win32(api, floating)
}

unsafe extern "C" fn get_preferred_api(
    _: *const clap_plugin,
    api: *mut *const c_char,
    floating: *mut bool,
) -> bool {
    let (Some(api), Some(floating)) = (unsafe { api.as_mut() }, unsafe { floating.as_mut() })
    else {
        return false;
    };

    *api = CLAP_WINDOW_API_WIN32.as_ptr();
    *floating = false;
    true
}

unsafe extern "C" fn create(
    plugin: *const clap_plugin,
    api: *const c_char,
    floating: bool,
) -> bool {
    let Some(core) = (unsafe { Core::from(plugin) }) else {
        return false;
    };
    if !win32(api, floating) {
        return false;
    }

    *core.gui.borrow_mut() = Some(Gui {
        panel: None,
        size: core.spec.info.size,
        scale: 1.0,
    });
    true
}

unsafe extern "C" fn destroy(plugin: *const clap_plugin) {
    if let Some(core) = unsafe { Core::from(plugin) } {
        core.gui.borrow_mut().take();
    }
}

unsafe extern "C" fn set_scale(plugin: *const clap_plugin, scale: f64) -> bool {
    let Some(core) = (unsafe { Core::from(plugin) }) else {
        return false;
    };
    if !(scale.is_finite() && scale > 0.0) {
        return false;
    }

    let mut gui = core.gui.borrow_mut();
    let Some(gui) = gui.as_mut() else {
        return false;
    };
    let grow = |v: u32| (v as f64 / gui.scale * scale).round() as u32;
    gui.size = (grow(gui.size.0), grow(gui.size.1));
    gui.scale = scale;
    true
}

unsafe extern "C" fn get_size(
    plugin: *const clap_plugin,
    width: *mut u32,
    height: *mut u32,
) -> bool {
    let (Some(core), Some(width), Some(height)) = (
        unsafe { Core::from(plugin) },
        unsafe { width.as_mut() },
        unsafe { height.as_mut() },
    ) else {
        return false;
    };
    let gui = core.gui.borrow();
    let Some(gui) = gui.as_ref() else {
        return false;
    };

    (*width, *height) = gui.size;
    true
}

unsafe extern "C" fn can_resize(_: *const clap_plugin) -> bool {
    true
}

unsafe extern "C" fn get_resize_hints(
    _: *const clap_plugin,
    hints: *mut clap_gui_resize_hints,
) -> bool {
    let Some(hints) = (unsafe { hints.as_mut() }) else {
        return false;
    };

    *hints = clap_gui_resize_hints {
        can_resize_horizontally: true,
        can_resize_vertically: true,
        preserve_aspect_ratio: false,
        aspect_ratio_width: 0,
        aspect_ratio_height: 0,
    };
    true
}

unsafe extern "C" fn adjust_size(
    plugin: *const clap_plugin,
    width: *mut u32,
    height: *mut u32,
) -> bool {
    let (Some(core), Some(width), Some(height)) = (
        unsafe { Core::from(plugin) },
        unsafe { width.as_mut() },
        unsafe { height.as_mut() },
    ) else {
        return false;
    };

    let scale = core.gui.borrow().as_ref().map_or(1.0, |g| g.scale);
    let (min_w, min_h) = core.spec.info.min_size;
    *width = (*width).max((min_w as f64 * scale) as u32);
    *height = (*height).max((min_h as f64 * scale) as u32);
    true
}

unsafe extern "C" fn set_size(plugin: *const clap_plugin, width: u32, height: u32) -> bool {
    let Some(core) = (unsafe { Core::from(plugin) }) else {
        return false;
    };
    let mut gui = core.gui.borrow_mut();
    let Some(gui) = gui.as_mut() else {
        return false;
    };

    gui.size = (width, height);
    if let Some(panel) = gui.panel.as_ref() {
        panel.resize((width as i32, height as i32));
    }
    true
}

unsafe extern "C" fn set_parent(plugin: *const clap_plugin, window: *const clap_window) -> bool {
    let (Some(core), Some(window)) = (unsafe { Core::from(plugin) }, unsafe { window.as_ref() })
    else {
        return false;
    };
    if !win32(window.api, false) {
        return false;
    }

    let parent = unsafe { window.specific.win32 };
    let mut gui = core.gui.borrow_mut();
    let Some(gui) = gui.as_mut() else {
        return false;
    };

    // Settings, then meters, kept current from what every thread shares.
    let count = core.spec.params.len();
    let values: Values = (0..core.spec.total())
        .map(|i| Cell::new(core.shared.value(i).unwrap_or(0.0)))
        .collect::<Vec<_>>()
        .into();
    let shown = values.clone();
    let shared = core.shared.clone();
    let pull = Box::new(move || {
        for (i, cell) in shown.iter().enumerate() {
            cell.set(match i < count {
                true => shared.value(i).unwrap_or(0.0),
                false => shared.meter(i - count),
            });
        }
    });
    let size = (gui.size.0 as i32, gui.size.1 as i32);
    gui.panel = Panel::open(core.spec, parent, size, values, gestures(core), pull);
    // The sound itself: here the editor and the audio thread share it.
    if let Some(panel) = &gui.panel {
        let scope = std::rc::Rc::new(std::cell::RefCell::new(Some(core.shared.scope())));
        panel.feed(kit_webui::feed::live(core.spec, scope.clone()));
        panel.hear(kit_webui::feed::learner(core.spec, scope));
    }
    gui.panel.is_some()
}

unsafe extern "C" fn set_transient(_: *const clap_plugin, _: *const clap_window) -> bool {
    false
}

unsafe extern "C" fn suggest_title(_: *const clap_plugin, _: *const c_char) {}

unsafe extern "C" fn show(_: *const clap_plugin) -> bool {
    true
}

unsafe extern "C" fn hide(_: *const clap_plugin) -> bool {
    true
}

static GUI: clap_plugin_gui = clap_plugin_gui {
    is_api_supported: Some(is_api_supported),
    get_preferred_api: Some(get_preferred_api),
    create: Some(create),
    destroy: Some(destroy),
    set_scale: Some(set_scale),
    get_size: Some(get_size),
    can_resize: Some(can_resize),
    get_resize_hints: Some(get_resize_hints),
    adjust_size: Some(adjust_size),
    set_size: Some(set_size),
    set_parent: Some(set_parent),
    set_transient: Some(set_transient),
    suggest_title: Some(suggest_title),
    show: Some(show),
    hide: Some(hide),
};

pub fn extension(id: &CStr) -> Option<*const c_void> {
    (id == CLAP_EXT_GUI).then_some(&GUI as *const _ as *const c_void)
}

// The editor as a host would open it, in a hidden window, with no host
// callbacks: a drag in the page must become a gesture for the host.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Descriptor;
    use crate::testing::no_host;
    use kit::{Asset, Block, Info, Param, Plugin, Role, range};
    use kit_webui::testing::{HostWindow, pump};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    const GAIN: u32 = 10;

    struct Gain;

    impl Plugin for Gain {
        const INFO: &'static Info = &Info {
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
        const PARAMS: &'static [Param] =
            &[range(GAIN, "gain", "Gain", "Level", (0.0, 2.0, 1.0), "")];
        const ASSETS: &'static [Asset] = &[(
            "/main.js",
            "text/javascript",
            "import { mount, auto } from './kit/kit.js'\nmount(auto)\n",
        )];

        fn new() -> Self {
            Self
        }

        fn prepare(&mut self, _rate: f32, _max_block: usize) {}

        fn process(&mut self, _block: &mut Block) {}
    }

    fn eval(core: &Core, js: &str) -> Option<String> {
        let out = Arc::new(Mutex::new(None));
        let slot = out.clone();
        let sent = core
            .gui
            .borrow()
            .as_ref()?
            .eval(js, move |r| *slot.lock().unwrap() = Some(r));
        if !sent {
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

    #[test]
    #[ignore = "opens a WebView2; npm run check runs it on Windows"]
    fn a_drag_in_the_editor_becomes_a_host_gesture() {
        let window_host = HostWindow::open();
        let desc = Descriptor::of::<Gain>();
        let host = no_host();
        let plugin = crate::create::<Gain>(desc.raw(), &host);
        let core = unsafe { Core::from(plugin) }.expect("plugin");
        let window = clap_window {
            api: CLAP_WINDOW_API_WIN32.as_ptr(),
            specific: clap_window_handle {
                win32: window_host.hwnd(),
            },
        };

        assert!(unsafe { create(plugin, CLAP_WINDOW_API_WIN32.as_ptr(), false) });
        assert!(
            !unsafe { create(plugin, c"cocoa".as_ptr(), false) },
            "win32 only"
        );
        assert!(unsafe { set_parent(plugin, &window) });

        let ready = "document.getElementById('app').dataset.ready";
        let until = Instant::now() + Duration::from_secs(20);
        while eval(core, ready).as_deref() != Some("\"true\"") {
            assert!(Instant::now() < until, "the page loads");
        }

        let gesture = format!(
            "for (const m of [{{type:'begin',id:{GAIN}}},{{type:'set',id:{GAIN},value:0.3}},{{type:'end',id:{GAIN}}}]) window.ipc.postMessage(JSON.stringify(m))"
        );
        eval(core, &gesture).expect("gesture");
        pump(100);
        assert_eq!(core.shared.value(0), Some(0.3));
        assert_eq!(
            core.shared.take_edit(0),
            kit::BEGIN | kit::SET | kit::END,
            "the whole gesture waits for the host's next flush"
        );

        // The editor first, then the plugin itself.
        unsafe {
            destroy(plugin);
            let free = (*plugin).destroy.expect("destroy");
            free(plugin);
        }
    }
}
