//! How the editor's side reaches the sound. In VST3 the processor and
//! the controller are two objects the host keeps apart, and the
//! controller hears of the processor only through parameters. A
//! waveform is not a parameter. So when the host connects the two,
//! the processor sends one message carrying a counted reference to
//! its `Scope`, and from then on the editor reads the sound directly,
//! with no lock and nothing more going through the host.
//!
//! That only works while both live in one process, which is how hosts
//! load them. The message says which process sent it, and one from
//! anywhere else is refused: the editor then simply has no waveform.

use kit::Scope;
use std::cell::Cell;
use std::ffi::{CStr, c_void};
use std::sync::Arc;
use vst3::Steinberg::Vst::*;
use vst3::Steinberg::*;
use vst3::{Class, ComRef, ComWrapper};

const ID: &CStr = c"kit.scope";
const PID: &CStr = c"pid";
const SCOPE: &CStr = c"scope";

/// The one message: who sent it, and the scope as a pointer that owns
/// one count of it.
struct Note {
    pid: i64,
    scope: i64,
    // This object as the attribute list it also is.
    attributes: Cell<*mut IAttributeList>,
}

impl Class for Note {
    type Interfaces = (IMessage, IAttributeList);
}

impl IMessageTrait for Note {
    unsafe fn getMessageID(&self) -> FIDString {
        ID.as_ptr()
    }

    unsafe fn setMessageID(&self, _id: FIDString) {}

    unsafe fn getAttributes(&self) -> *mut IAttributeList {
        self.attributes.get()
    }
}

impl IAttributeListTrait for Note {
    unsafe fn setInt(&self, _id: IAttributeList_::AttrID, _value: int64) -> tresult {
        kResultFalse
    }

    unsafe fn getInt(&self, id: IAttributeList_::AttrID, value: *mut int64) -> tresult {
        let (false, Some(value)) = (id.is_null(), unsafe { value.as_mut() }) else {
            return kInvalidArgument;
        };
        let name = unsafe { CStr::from_ptr(id) };
        *value = match name {
            n if n == PID => self.pid,
            n if n == SCOPE => self.scope,
            _ => return kResultFalse,
        };
        kResultOk
    }

    unsafe fn setFloat(&self, _id: IAttributeList_::AttrID, _value: f64) -> tresult {
        kResultFalse
    }

    unsafe fn getFloat(&self, _id: IAttributeList_::AttrID, _value: *mut f64) -> tresult {
        kResultFalse
    }

    unsafe fn setString(&self, _id: IAttributeList_::AttrID, _string: *const TChar) -> tresult {
        kResultFalse
    }

    unsafe fn getString(
        &self,
        _id: IAttributeList_::AttrID,
        _string: *mut TChar,
        _size: uint32,
    ) -> tresult {
        kResultFalse
    }

    unsafe fn setBinary(
        &self,
        _id: IAttributeList_::AttrID,
        _data: *const c_void,
        _size: uint32,
    ) -> tresult {
        kResultFalse
    }

    unsafe fn getBinary(
        &self,
        _id: IAttributeList_::AttrID,
        _data: *mut *const c_void,
        _size: *mut uint32,
    ) -> tresult {
        kResultFalse
    }
}

/// From the processor: offers `scope` to whatever is at the other end
/// of the connection. Whether it was taken.
pub fn offer(other: &ComRef<IConnectionPoint>, scope: Arc<Scope>) -> bool {
    let raw = Arc::into_raw(scope);
    let note = ComWrapper::new(Note {
        pid: i64::from(std::process::id()),
        scope: raw as i64,
        attributes: Cell::new(std::ptr::null_mut()),
    });
    let parts = (
        note.to_com_ptr::<IMessage>(),
        note.to_com_ptr::<IAttributeList>(),
    );
    let taken = match &parts {
        (Some(message), Some(attributes)) => {
            note.attributes.set(attributes.as_ptr());
            unsafe { other.notify(message.as_ptr()) == kResultOk }
        }
        _ => false,
    };
    // Not taken: the count the message carried is ours to give back.
    if !taken {
        drop(unsafe { Arc::from_raw(raw) });
    }
    taken
}

/// From the controller: the scope a message carries, if it is ours and
/// from this process. Takes the count the message carried, so a
/// message must be accepted once.
pub fn accept(message: *mut IMessage) -> Option<Arc<Scope>> {
    let message = unsafe { ComRef::from_raw(message) }?;
    let id = unsafe { message.getMessageID() };
    if id.is_null() || unsafe { CStr::from_ptr(id) } != ID {
        return None;
    }
    let attributes = unsafe { ComRef::from_raw(message.getAttributes()) }?;
    let int = |name: &CStr| {
        let mut value = 0;
        (unsafe { attributes.getInt(name.as_ptr(), &mut value) } == kResultOk).then_some(value)
    };
    if int(PID)? != i64::from(std::process::id()) {
        return None;
    }
    let raw = int(SCOPE)? as *const Scope;
    (!raw.is_null()).then(|| unsafe { Arc::from_raw(raw) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::Controller;
    use kit::{Info, Role, Spec};

    const INFO: Info = Info {
        name: "Thing",
        vendor: "kit",
        url: "",
        description: "",
        version: "1.2.3",
        clap_id: "dev.kit.thing",
        vst3_processor: [0; 4],
        vst3_controller: [1; 4],
        role: Role::Effect,
        audio_in: 2,
        audio_out: 2,
        midi_in: false,
        midi_out: false,
        size: (1, 1),
        min_size: (1, 1),
    };
    const SPEC: Spec = Spec {
        info: &INFO,
        params: &[],
        meters: &[],
        assets: &[],
    };

    fn note(pid: i64, scope: &Arc<Scope>) -> (ComWrapper<Note>, vst3::ComPtr<IMessage>) {
        let note = ComWrapper::new(Note {
            pid,
            scope: Arc::as_ptr(scope) as i64,
            attributes: Cell::new(std::ptr::null_mut()),
        });
        let attributes = note.to_com_ptr::<IAttributeList>().unwrap();
        note.attributes.set(attributes.as_ptr());
        let message = note.to_com_ptr::<IMessage>().unwrap();
        (note, message)
    }

    #[test]
    fn the_controller_takes_the_scope_the_processor_offers() {
        let controller = ComWrapper::new(Controller::new(SPEC));
        let point = controller.to_com_ptr::<IConnectionPoint>().unwrap();
        let scope = Arc::new(Scope::default());
        scope.push(0.25, 0.5);

        assert!(offer(&point.as_com_ref(), scope.clone()));

        // One count is ours and one the controller's: nothing leaked,
        // nothing freed early, and it is the same scope.
        assert_eq!(Arc::strong_count(&scope), 2);
        let theirs = controller.scope.borrow().clone().expect("a scope");
        assert_eq!(theirs.written(), 1);
        drop(theirs);
        drop(controller);
        drop(point);
        assert_eq!(Arc::strong_count(&scope), 1);
    }

    #[test]
    fn a_message_from_another_process_is_refused() {
        let scope = Arc::new(Scope::default());
        let (_keep, message) = note(i64::from(std::process::id()) + 1, &scope);

        assert!(accept(message.as_ptr()).is_none());
        assert_eq!(Arc::strong_count(&scope), 1);
    }

    #[test]
    fn an_offer_nobody_takes_gives_its_count_back() {
        // A connection point that refuses everything: the processor's
        // own, which has nothing to learn from a message.
        struct Deaf;
        impl Class for Deaf {
            type Interfaces = (IConnectionPoint,);
        }
        impl IConnectionPointTrait for Deaf {
            unsafe fn connect(&self, _other: *mut IConnectionPoint) -> tresult {
                kResultOk
            }
            unsafe fn disconnect(&self, _other: *mut IConnectionPoint) -> tresult {
                kResultOk
            }
            unsafe fn notify(&self, _message: *mut IMessage) -> tresult {
                kResultFalse
            }
        }
        let deaf = ComWrapper::new(Deaf);
        let point = deaf.to_com_ptr::<IConnectionPoint>().unwrap();
        let scope = Arc::new(Scope::default());

        assert!(!offer(&point.as_com_ref(), scope.clone()));
        assert_eq!(Arc::strong_count(&scope), 1);
    }
}
