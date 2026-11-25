//! Any kit plugin as a VST3: a factory with two classes, the audio
//! side and the host-facing side, both written once here against the
//! plugin's spec. A plugin's crate says `kit_vst3::export!(Mine);`.

// The SDK's enums are i32 on Windows and u32 elsewhere, so their casts
// are needed on one platform and not on the other.
#![allow(clippy::unnecessary_cast)]

mod connect;
mod controller;
#[cfg(windows)]
mod editor;
mod events;
mod processor;
mod strings;
pub mod testing;
mod transport;

pub use controller::Controller;
pub use processor::Processor;

use kit::{Plugin, Role, Spec};
use std::ffi::c_void;
use std::marker::PhantomData;
use strings::copy_cstring;
use vst3::Steinberg::*;
use vst3::{Class, ComWrapper, uid};

const SDK_VERSION: &str = "VST 3.8.0";

pub(crate) fn class_id(id: [u32; 4]) -> TUID {
    uid(id[0], id[1], id[2], id[3])
}

/// The plugin's factory: what a host asks the library for first.
pub struct Factory<P: Plugin>(PhantomData<fn() -> P>);

impl<P: Plugin> Default for Factory<P> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<P: Plugin> Class for Factory<P> {
    type Interfaces = (IPluginFactory2,);
}

// (class id, category, subcategories) for each class, by index.
fn classes(spec: &Spec) -> [(TUID, &'static str, &'static str); 2] {
    let sub = match spec.info.role {
        Role::Instrument => "Instrument",
        Role::Effect => "Fx",
    };
    [
        (
            class_id(spec.info.vst3_processor),
            "Audio Module Class",
            sub,
        ),
        (
            class_id(spec.info.vst3_controller),
            "Component Controller Class",
            "",
        ),
    ]
}

impl<P: Plugin> IPluginFactoryTrait for Factory<P> {
    unsafe fn getFactoryInfo(&self, info: *mut PFactoryInfo) -> tresult {
        let Some(info) = (unsafe { info.as_mut() }) else {
            return kInvalidArgument;
        };

        copy_cstring(P::INFO.vendor, &mut info.vendor);
        copy_cstring(P::INFO.url, &mut info.url);
        copy_cstring("", &mut info.email);
        info.flags = PFactoryInfo_::FactoryFlags_::kUnicode as int32;
        kResultOk
    }

    unsafe fn countClasses(&self) -> i32 {
        2
    }

    unsafe fn getClassInfo(&self, index: i32, info: *mut PClassInfo) -> tresult {
        let classes = classes(&Spec::of::<P>());
        let (Some(&(cid, category, _)), Some(info)) =
            (classes.get(index as usize), unsafe { info.as_mut() })
        else {
            return kInvalidArgument;
        };

        info.cid = cid;
        info.cardinality = PClassInfo_::ClassCardinality_::kManyInstances as int32;
        copy_cstring(category, &mut info.category);
        copy_cstring(P::INFO.name, &mut info.name);
        kResultOk
    }

    unsafe fn createInstance(
        &self,
        cid: FIDString,
        iid: FIDString,
        obj: *mut *mut c_void,
    ) -> tresult {
        if cid.is_null() || obj.is_null() {
            return kInvalidArgument;
        }

        let wanted = unsafe { *(cid as *const TUID) };
        let instance = if wanted == class_id(P::INFO.vst3_processor) {
            ComWrapper::new(Processor::<P>::new()).to_com_ptr::<FUnknown>()
        } else if wanted == class_id(P::INFO.vst3_controller) {
            ComWrapper::new(Controller::new(Spec::of::<P>())).to_com_ptr::<FUnknown>()
        } else {
            None
        };
        let Some(instance) = instance else {
            unsafe { *obj = std::ptr::null_mut() };
            return kNoInterface;
        };

        let ptr = instance.as_ptr();
        unsafe { ((*(*ptr).vtbl).queryInterface)(ptr, iid as *mut TUID, obj) }
    }
}

impl<P: Plugin> IPluginFactory2Trait for Factory<P> {
    unsafe fn getClassInfo2(&self, index: i32, info: *mut PClassInfo2) -> tresult {
        let classes = classes(&Spec::of::<P>());
        let (Some(&(cid, category, sub)), Some(info)) =
            (classes.get(index as usize), unsafe { info.as_mut() })
        else {
            return kInvalidArgument;
        };

        info.cid = cid;
        info.cardinality = PClassInfo_::ClassCardinality_::kManyInstances as int32;
        copy_cstring(category, &mut info.category);
        copy_cstring(P::INFO.name, &mut info.name);
        info.classFlags = 0;
        copy_cstring(sub, &mut info.subCategories);
        copy_cstring(P::INFO.vendor, &mut info.vendor);
        copy_cstring(P::INFO.version, &mut info.version);
        copy_cstring(SDK_VERSION, &mut info.sdkVersion);
        kResultOk
    }
}

/// The factory as the pointer `GetPluginFactory` returns.
pub fn factory<P: Plugin>() -> *mut IPluginFactory {
    ComWrapper::new(Factory::<P>::default())
        .to_com_ptr::<IPluginFactory>()
        .map_or(std::ptr::null_mut(), |p| p.into_raw())
}

/// Call from the library's exit: frees what the editor registered.
pub fn unload() {
    kit_webui::unregister();
}

/// The entry points a VST3 host looks for, for plugin type `$plugin`.
#[macro_export]
macro_rules! export {
    ($plugin:ty) => {
        #[cfg(target_os = "windows")]
        #[unsafe(no_mangle)]
        extern "system" fn InitDll() -> bool {
            true
        }

        #[cfg(target_os = "windows")]
        #[unsafe(no_mangle)]
        extern "system" fn ExitDll() -> bool {
            $crate::unload();
            true
        }

        #[cfg(target_os = "linux")]
        #[unsafe(no_mangle)]
        extern "system" fn ModuleEntry(_: *mut ::std::ffi::c_void) -> bool {
            true
        }

        #[cfg(target_os = "linux")]
        #[unsafe(no_mangle)]
        extern "system" fn ModuleExit() -> bool {
            true
        }

        #[cfg(target_os = "macos")]
        #[unsafe(no_mangle)]
        extern "system" fn BundleEntry(_: *mut ::std::ffi::c_void) -> bool {
            true
        }

        #[cfg(target_os = "macos")]
        #[unsafe(no_mangle)]
        extern "system" fn BundleExit() -> bool {
            true
        }

        #[unsafe(no_mangle)]
        extern "system" fn GetPluginFactory() -> *mut ::std::ffi::c_void {
            $crate::factory::<$plugin>().cast()
        }
    };
}
