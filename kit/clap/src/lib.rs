//! Any kit plugin as a CLAP, on raw `clap-sys` bindings. Everything a
//! host asks about (ports, parameters, state, the editor) is answered
//! from the plugin's spec; only making the instance and running it
//! know the plugin's type. A plugin's crate says
//! `kit_clap::export!(Mine);`.

#[cfg(windows)]
mod gui;
mod instance;
mod params;
mod ports;
pub mod testing;
mod transport;

pub use clap_sys as sys;
pub use instance::create;

use clap_sys::plugin::clap_plugin_descriptor;
use clap_sys::plugin_features::{
    CLAP_PLUGIN_FEATURE_AUDIO_EFFECT, CLAP_PLUGIN_FEATURE_INSTRUMENT, CLAP_PLUGIN_FEATURE_STEREO,
};
use clap_sys::version::CLAP_VERSION;
use kit::{Plugin, Role, Spec};
use std::ffi::{CString, c_char};

/// What a host reads about the plugin before making one: C strings
/// made once from the spec and kept for the life of the library.
pub struct Descriptor {
    raw: clap_plugin_descriptor,
    id: CString,
    // Kept alive: `raw` points into them.
    _strings: Vec<CString>,
    _features: Box<[*const c_char]>,
}

// Raw pointers to strings it owns and never changes.
unsafe impl Send for Descriptor {}
unsafe impl Sync for Descriptor {}

fn c(text: &str) -> CString {
    CString::new(text.replace('\0', "")).unwrap_or_default()
}

impl Descriptor {
    pub fn of<P: Plugin>() -> Self {
        let info = Spec::of::<P>().info;
        let id = c(info.clap_id);
        let strings: Vec<CString> = [
            info.name,
            info.vendor,
            info.url,
            info.version,
            info.description,
            "",
        ]
        .into_iter()
        .map(c)
        .collect();
        let kind = match info.role {
            Role::Effect => CLAP_PLUGIN_FEATURE_AUDIO_EFFECT,
            Role::Instrument => CLAP_PLUGIN_FEATURE_INSTRUMENT,
        };
        let features: Box<[*const c_char]> = Box::new([
            kind.as_ptr(),
            CLAP_PLUGIN_FEATURE_STEREO.as_ptr(),
            std::ptr::null(),
        ]);
        let raw = clap_plugin_descriptor {
            clap_version: CLAP_VERSION,
            id: id.as_ptr(),
            name: strings[0].as_ptr(),
            vendor: strings[1].as_ptr(),
            url: strings[2].as_ptr(),
            manual_url: strings[5].as_ptr(),
            support_url: strings[5].as_ptr(),
            version: strings[3].as_ptr(),
            description: strings[4].as_ptr(),
            features: features.as_ptr(),
        };
        Self {
            raw,
            id,
            _strings: strings,
            _features: features,
        }
    }

    pub fn raw(&self) -> *const clap_plugin_descriptor {
        &self.raw
    }

    pub fn id(&self) -> &std::ffi::CStr {
        &self.id
    }
}

/// Call from the library's deinit: frees what the editor registered.
pub fn unload() {
    kit_webui::unregister();
}

/// The `clap_entry` a CLAP host looks for, for plugin type `$plugin`.
#[macro_export]
macro_rules! export {
    ($plugin:ty) => {
        mod __kit_clap {
            use super::*;
            use ::std::ffi::{CStr, c_char, c_void};
            use ::std::sync::OnceLock;
            use $crate::sys::entry::clap_plugin_entry;
            use $crate::sys::factory::plugin_factory::{
                CLAP_PLUGIN_FACTORY_ID, clap_plugin_factory,
            };
            use $crate::sys::host::clap_host;
            use $crate::sys::plugin::{clap_plugin, clap_plugin_descriptor};
            use $crate::sys::version::CLAP_VERSION;

            fn described() -> &'static $crate::Descriptor {
                static DESCRIPTOR: OnceLock<$crate::Descriptor> = OnceLock::new();
                DESCRIPTOR.get_or_init($crate::Descriptor::of::<$plugin>)
            }

            unsafe extern "C" fn count(_: *const clap_plugin_factory) -> u32 {
                1
            }

            unsafe extern "C" fn descriptor(
                _: *const clap_plugin_factory,
                index: u32,
            ) -> *const clap_plugin_descriptor {
                match index {
                    0 => described().raw(),
                    _ => ::std::ptr::null(),
                }
            }

            unsafe extern "C" fn create(
                _: *const clap_plugin_factory,
                host: *const clap_host,
                id: *const c_char,
            ) -> *const clap_plugin {
                let ours = !id.is_null() && unsafe { CStr::from_ptr(id) } == described().id();
                if host.is_null() || !ours {
                    return ::std::ptr::null();
                }

                $crate::create::<$plugin>(described().raw(), host)
            }

            static FACTORY: clap_plugin_factory = clap_plugin_factory {
                get_plugin_count: Some(count),
                get_plugin_descriptor: Some(descriptor),
                create_plugin: Some(create),
            };

            unsafe extern "C" fn init(_: *const c_char) -> bool {
                true
            }

            unsafe extern "C" fn deinit() {
                $crate::unload();
            }

            unsafe extern "C" fn factory(id: *const c_char) -> *const c_void {
                if !id.is_null() && unsafe { CStr::from_ptr(id) } == CLAP_PLUGIN_FACTORY_ID {
                    return &FACTORY as *const _ as *const c_void;
                }
                ::std::ptr::null()
            }

            #[allow(non_upper_case_globals)]
            #[unsafe(no_mangle)]
            pub static clap_entry: clap_plugin_entry = clap_plugin_entry {
                clap_version: CLAP_VERSION,
                init: Some(init),
                deinit: Some(deinit),
                get_factory: Some(factory),
            };
        }
    };
}
