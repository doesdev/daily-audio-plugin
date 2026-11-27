//! Parameters and saved state, both straight from the spec. CLAP
//! values are plain: a list's index, a range in its own units.

use crate::instance::{Core, exchange};
use clap_sys::events::{clap_input_events, clap_output_events};
use clap_sys::ext::params::*;
use clap_sys::ext::state::{CLAP_EXT_STATE, clap_plugin_state};
use clap_sys::id::clap_id;
use clap_sys::plugin::clap_plugin;
use clap_sys::stream::{clap_istream, clap_ostream};
use kit::{Param, state};
use std::ffi::{CStr, c_char, c_void};

fn flags(p: &Param) -> clap_param_info_flags {
    let stepped = if p.steps() > 0 {
        CLAP_PARAM_IS_STEPPED
    } else {
        0
    };
    let list = if p.is_list() { CLAP_PARAM_IS_ENUM } else { 0 };
    // The host's own bypass button and the plugin's are then one.
    let bypass = if p.is_bypass() {
        CLAP_PARAM_IS_BYPASS
    } else {
        0
    };
    CLAP_PARAM_IS_AUTOMATABLE | stepped | list | bypass
}

// Into a fixed C string, cut short if need be, always terminated.
fn copy(text: &str, out: &mut [c_char]) {
    let Some(room) = out.len().checked_sub(1) else {
        return;
    };

    let bytes = &text.as_bytes()[..text.len().min(room)];
    for (slot, &b) in out.iter_mut().zip(bytes) {
        *slot = b as c_char;
    }
    out[bytes.len()] = 0;
}

unsafe extern "C" fn count(plugin: *const clap_plugin) -> u32 {
    unsafe { Core::from(plugin) }.map_or(0, |core| core.spec.params.len() as u32)
}

unsafe extern "C" fn get_info(
    plugin: *const clap_plugin,
    index: u32,
    info: *mut clap_param_info,
) -> bool {
    let (Some(core), Some(info)) = (unsafe { Core::from(plugin) }, unsafe { info.as_mut() }) else {
        return false;
    };
    let Some(p) = core.spec.params.get(index as usize) else {
        return false;
    };

    info.id = p.id;
    info.flags = flags(p);
    info.cookie = std::ptr::null_mut();
    copy(p.title, &mut info.name);
    copy(p.group, &mut info.module);
    info.min_value = p.plain(0.0);
    info.max_value = p.plain(1.0);
    info.default_value = p.default;
    true
}

unsafe extern "C" fn get_value(plugin: *const clap_plugin, id: clap_id, out: *mut f64) -> bool {
    let (Some(core), Some(out)) = (unsafe { Core::from(plugin) }, unsafe { out.as_mut() }) else {
        return false;
    };
    let Some(index) = core.spec.setting_of(id) else {
        return false;
    };
    let Some(norm) = core.shared.value(index) else {
        return false;
    };

    *out = core.spec.params[index].plain(norm);
    true
}

unsafe extern "C" fn value_to_text(
    plugin: *const clap_plugin,
    id: clap_id,
    value: f64,
    out: *mut c_char,
    capacity: u32,
) -> bool {
    let Some(p) = unsafe { Core::from(plugin) }.and_then(|c| c.setting(id)) else {
        return false;
    };
    if out.is_null() || capacity == 0 {
        return false;
    }

    let out = unsafe { std::slice::from_raw_parts_mut(out, capacity as usize) };
    copy(&p.display(p.normalized(value)), out);
    true
}

unsafe extern "C" fn text_to_value(
    plugin: *const clap_plugin,
    id: clap_id,
    text: *const c_char,
    out: *mut f64,
) -> bool {
    let p = unsafe { Core::from(plugin) }.and_then(|c| c.setting(id));
    let (Some(p), Some(out)) = (p, unsafe { out.as_mut() }) else {
        return false;
    };
    if text.is_null() {
        return false;
    }

    let text = unsafe { CStr::from_ptr(text) }.to_string_lossy();
    let Some(norm) = p.parse(&text) else {
        return false;
    };
    *out = p.plain(norm);
    true
}

unsafe extern "C" fn flush(
    plugin: *const clap_plugin,
    in_events: *const clap_input_events,
    out: *const clap_output_events,
) {
    if let Some(core) = unsafe { Core::from(plugin) } {
        exchange(core, in_events, out);
    }
}

static PARAMS_EXT: clap_plugin_params = clap_plugin_params {
    count: Some(count),
    get_info: Some(get_info),
    get_value: Some(get_value),
    value_to_text: Some(value_to_text),
    text_to_value: Some(text_to_value),
    flush: Some(flush),
};

unsafe extern "C" fn save(plugin: *const clap_plugin, stream: *const clap_ostream) -> bool {
    let (Some(core), Some(stream)) = (unsafe { Core::from(plugin) }, unsafe { stream.as_ref() })
    else {
        return false;
    };
    let Some(write) = stream.write else {
        return false;
    };

    let bytes = state::encode_linked(
        &core.spec,
        &core.shared.values(),
        &core.shared.learnt(),
        &core.shared.links(),
    );
    let mut at = 0;
    while at < bytes.len() {
        let rest = &bytes[at..];
        let n = unsafe { write(stream, rest.as_ptr() as *const c_void, rest.len() as u64) };
        if n <= 0 {
            return false;
        }
        at += n as usize;
    }
    true
}

unsafe extern "C" fn load(plugin: *const clap_plugin, stream: *const clap_istream) -> bool {
    let (Some(core), Some(stream)) = (unsafe { Core::from(plugin) }, unsafe { stream.as_ref() })
    else {
        return false;
    };
    let Some(read) = stream.read else {
        return false;
    };

    let mut bytes = Vec::new();
    let mut buf = [0u8; 256];
    loop {
        let n = unsafe { read(stream, buf.as_mut_ptr() as *mut c_void, buf.len() as u64) };
        if n < 0 {
            return false;
        }
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&buf[..n as usize]);
    }

    let Some(values) = state::decode(&core.spec, &bytes) else {
        return false;
    };
    core.shared.set_all(&values);
    core.shared.set_learnt(&state::learnt(&core.spec, &bytes));
    core.shared.set_links(&state::linked(&core.spec, &bytes));
    core.rescan_values();
    true
}

static STATE_EXT: clap_plugin_state = clap_plugin_state {
    save: Some(save),
    load: Some(load),
};

pub fn extension(id: &CStr) -> Option<*const c_void> {
    if id == CLAP_EXT_PARAMS {
        return Some(&PARAMS_EXT as *const _ as *const c_void);
    }
    if id == CLAP_EXT_STATE {
        return Some(&STATE_EXT as *const _ as *const c_void);
    }
    None
}
