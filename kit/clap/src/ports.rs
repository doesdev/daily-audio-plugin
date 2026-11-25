//! The audio and note ports the spec says the plugin has: at most one
//! of each kind each way, processed in place where the host likes.

use crate::instance::Core;
use clap_sys::ext::audio_ports::*;
use clap_sys::ext::note_ports::*;
use clap_sys::id::CLAP_INVALID_ID;
use clap_sys::plugin::clap_plugin;
use std::ffi::{CStr, c_char, c_void};

fn copy(text: &CStr, out: &mut [c_char]) {
    let bytes = text.to_bytes_with_nul();
    for (slot, &b) in out.iter_mut().zip(bytes) {
        *slot = b as c_char;
    }
}

fn channels(core: &Core, is_input: bool) -> u32 {
    match is_input {
        true => core.spec.info.audio_in,
        false => core.spec.info.audio_out,
    }
}

unsafe extern "C" fn audio_count(plugin: *const clap_plugin, is_input: bool) -> u32 {
    let Some(core) = (unsafe { Core::from(plugin) }) else {
        return 0;
    };
    u32::from(channels(core, is_input) > 0)
}

unsafe extern "C" fn audio_get(
    plugin: *const clap_plugin,
    index: u32,
    is_input: bool,
    info: *mut clap_audio_port_info,
) -> bool {
    let (Some(core), Some(info)) = (unsafe { Core::from(plugin) }, unsafe { info.as_mut() }) else {
        return false;
    };
    let count = channels(core, is_input);
    if index != 0 || count == 0 {
        return false;
    }

    info.id = 0;
    copy(if is_input { c"Input" } else { c"Output" }, &mut info.name);
    info.flags = CLAP_AUDIO_PORT_IS_MAIN;
    info.channel_count = count;
    info.port_type = match count {
        1 => CLAP_PORT_MONO.as_ptr(),
        2 => CLAP_PORT_STEREO.as_ptr(),
        _ => std::ptr::null(),
    };
    // In place only when the other side is the same shape.
    let pair = channels(core, !is_input) == count;
    info.in_place_pair = if pair { 0 } else { CLAP_INVALID_ID };
    true
}

static AUDIO: clap_plugin_audio_ports = clap_plugin_audio_ports {
    count: Some(audio_count),
    get: Some(audio_get),
};

fn notes(core: &Core, is_input: bool) -> bool {
    match is_input {
        true => core.spec.info.midi_in,
        false => core.spec.info.midi_out,
    }
}

unsafe extern "C" fn note_count(plugin: *const clap_plugin, is_input: bool) -> u32 {
    let Some(core) = (unsafe { Core::from(plugin) }) else {
        return 0;
    };
    u32::from(notes(core, is_input))
}

unsafe extern "C" fn note_get(
    plugin: *const clap_plugin,
    index: u32,
    is_input: bool,
    info: *mut clap_note_port_info,
) -> bool {
    let (Some(core), Some(info)) = (unsafe { Core::from(plugin) }, unsafe { info.as_mut() }) else {
        return false;
    };
    if index != 0 || !notes(core, is_input) {
        return false;
    }

    info.id = 0;
    info.supported_dialects = match is_input {
        true => CLAP_NOTE_DIALECT_CLAP | CLAP_NOTE_DIALECT_MIDI,
        false => CLAP_NOTE_DIALECT_MIDI,
    };
    info.preferred_dialect = CLAP_NOTE_DIALECT_MIDI;
    copy(
        if is_input { c"MIDI in" } else { c"MIDI out" },
        &mut info.name,
    );
    true
}

static NOTES: clap_plugin_note_ports = clap_plugin_note_ports {
    count: Some(note_count),
    get: Some(note_get),
};

pub fn extension(id: &CStr) -> Option<*const c_void> {
    if id == CLAP_EXT_AUDIO_PORTS {
        return Some(&AUDIO as *const _ as *const c_void);
    }
    if id == CLAP_EXT_NOTE_PORTS {
        return Some(&NOTES as *const _ as *const c_void);
    }
    None
}
