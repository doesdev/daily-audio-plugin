//! A host for tests: makes a plugin the way a CLAP host does and plays
//! it a block at a time. Everything a block needs is made up front, so
//! `block` itself never allocates and an allocation counted across it
//! is the plugin's.

use crate::{Descriptor, create};
use clap_sys::audio_buffer::clap_audio_buffer;
use clap_sys::events::*;
use clap_sys::ext::audio_ports::*;
use clap_sys::ext::latency::{CLAP_EXT_LATENCY, clap_plugin_latency};
use clap_sys::ext::params::{CLAP_EXT_PARAMS, clap_param_info, clap_plugin_params};
use clap_sys::ext::state::{CLAP_EXT_STATE, clap_plugin_state};
use clap_sys::ext::track_info::*;
use clap_sys::host::clap_host;
use clap_sys::plugin::clap_plugin;
use clap_sys::process::{CLAP_PROCESS_ERROR, clap_process};
use clap_sys::stream::{clap_istream, clap_ostream};
use clap_sys::version::CLAP_VERSION;
use kit::{Event, Plugin, Spec};
use std::cell::RefCell;
use std::ffi::{CStr, c_char, c_void};

// An event the host hands in: each starts with its header.
#[derive(Clone, Copy)]
enum In {
    Value(clap_event_param_value),
    Midi(clap_event_midi),
}

/// Something the plugin sent out of a block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Out {
    Midi(u32, [u8; 3]),
    Begin(u32),
    /// A setting's host id and its plain value.
    Value(u32, f64),
    End(u32),
}

struct Lists {
    input: Vec<In>,
    output: Vec<Out>,
}

unsafe extern "C" fn in_size(list: *const clap_input_events) -> u32 {
    let lists = unsafe { &*((*list).ctx as *const Lists) };
    lists.input.len() as u32
}

unsafe extern "C" fn in_get(
    list: *const clap_input_events,
    index: u32,
) -> *const clap_event_header {
    let lists = unsafe { &*((*list).ctx as *const Lists) };
    match lists.input.get(index as usize) {
        Some(In::Value(e)) => &e.header,
        Some(In::Midi(e)) => &e.header,
        None => std::ptr::null(),
    }
}

unsafe extern "C" fn out_push(
    list: *const clap_output_events,
    event: *const clap_event_header,
) -> bool {
    let lists = unsafe { &mut *((*list).ctx as *mut Lists) };
    let Some(head) = (unsafe { event.as_ref() }) else {
        return false;
    };
    if lists.output.len() == lists.output.capacity() {
        return false;
    }

    let gesture = || unsafe { &*(event as *const clap_event_param_gesture) }.param_id;
    let seen = match head.type_ {
        CLAP_EVENT_MIDI => {
            let m = unsafe { &*(event as *const clap_event_midi) };
            Out::Midi(head.time, m.data)
        }
        CLAP_EVENT_PARAM_VALUE => {
            let v = unsafe { &*(event as *const clap_event_param_value) };
            Out::Value(v.param_id, v.value)
        }
        CLAP_EVENT_PARAM_GESTURE_BEGIN => Out::Begin(gesture()),
        CLAP_EVENT_PARAM_GESTURE_END => Out::End(gesture()),
        _ => return true,
    };
    lists.output.push(seen);
    true
}

unsafe extern "C" fn no_extension(_: *const clap_host, _: *const c_char) -> *const c_void {
    std::ptr::null()
}

unsafe extern "C" fn nothing(_: *const clap_host) {}

// A host that names the plugin's track: `host_data` is the name.
static TRACKS: clap_host_track_info = clap_host_track_info {
    get: Some(track_of),
};

unsafe extern "C" fn track_of(host: *const clap_host, info: *mut clap_track_info) -> bool {
    let name = unsafe { ((*host).host_data as *const RefCell<String>).as_ref() };
    let (Some(name), Some(info)) = (name, unsafe { info.as_mut() }) else {
        return false;
    };
    let name = name.borrow();
    if name.is_empty() {
        return false;
    }
    info.flags = CLAP_TRACK_INFO_HAS_TRACK_NAME;
    let room = info.name.len() - 1;
    for (slot, byte) in info.name.iter_mut().zip(name.bytes().take(room)) {
        *slot = byte as c_char;
    }
    info.name[name.len().min(room)] = 0;
    true
}

unsafe extern "C" fn names_tracks(_: *const clap_host, id: *const c_char) -> *const c_void {
    match unsafe { CStr::from_ptr(id) } == CLAP_EXT_TRACK_INFO {
        true => &TRACKS as *const _ as *const c_void,
        false => std::ptr::null(),
    }
}

unsafe extern "C" fn write(stream: *const clap_ostream, buffer: *const c_void, size: u64) -> i64 {
    let bytes = unsafe { &mut *((*stream).ctx as *mut Vec<u8>) };
    bytes.extend_from_slice(unsafe { std::slice::from_raw_parts(buffer.cast(), size as usize) });
    size as i64
}

unsafe extern "C" fn read(stream: *const clap_istream, buffer: *mut c_void, size: u64) -> i64 {
    let (bytes, at) = unsafe { &mut *((*stream).ctx as *mut (Vec<u8>, usize)) };
    let take = (size as usize).min(bytes.len() - *at);
    unsafe { std::ptr::copy_nonoverlapping(bytes[*at..].as_ptr(), buffer.cast(), take) };
    *at += take;
    take as i64
}

/// A host that offers nothing, for a plugin made outside `Host`.
pub fn no_host() -> clap_host {
    clap_host {
        clap_version: CLAP_VERSION,
        host_data: std::ptr::null_mut(),
        name: c"kit test host".as_ptr(),
        vendor: c"kit".as_ptr(),
        url: c"".as_ptr(),
        version: c"0".as_ptr(),
        get_extension: Some(no_extension),
        request_restart: Some(nothing),
        request_process: Some(nothing),
        request_callback: Some(nothing),
    }
}

fn header<T>(time: u32, kind: u16) -> clap_event_header {
    clap_event_header {
        size: size_of::<T>() as u32,
        time,
        space_id: CLAP_CORE_EVENT_SPACE_ID,
        type_: kind,
        flags: 0,
    }
}

/// A plugin made through `create` and activated.
pub struct Host {
    pub spec: Spec,
    plugin: *const clap_plugin,
    // Never resized: the pointers below are into them.
    input: Vec<Vec<f32>>,
    output: Vec<Vec<f32>>,
    in_ptrs: Vec<*mut f32>,
    out_ptrs: Vec<*mut f32>,
    key: Vec<Vec<f32>>,
    key_ptrs: Vec<*mut f32>,
    lists: Box<Lists>,
    // Kept alive: the plugin holds pointers to both.
    _host: Box<clap_host>,
    // The track the host says the plugin is on.
    track: Box<RefCell<String>>,
    _desc: Box<Descriptor>,
}

impl Host {
    /// Makes plugin `P` at `rate`, promised blocks of at most `block`.
    pub fn load<P: Plugin>(rate: f32, block: usize) -> Self {
        Self::load_promised::<P>(rate, block, block)
    }

    /// Makes plugin `P` promised blocks of at most `promised`, with
    /// room to hand it up to `block` all the same.
    pub fn load_promised<P: Plugin>(rate: f32, promised: usize, block: usize) -> Self {
        let spec = Spec::of::<P>();
        let desc = Box::new(Descriptor::of::<P>());
        let track = Box::new(RefCell::new(String::new()));
        let host = Box::new(clap_host {
            host_data: &*track as *const RefCell<String> as *mut c_void,
            get_extension: Some(names_tracks),
            ..no_host()
        });
        let plugin = create::<P>(desc.raw(), &*host);
        let raw = unsafe { plugin.as_ref() }.expect("a plugin");
        unsafe {
            assert!(raw.init.expect("init")(plugin));
            assert!(raw.activate.expect("activate")(
                plugin,
                rate as f64,
                1,
                promised as u32
            ));
            assert!(raw.start_processing.expect("start")(plugin));
        }

        let buffers = |n: u32| -> Vec<Vec<f32>> { (0..n).map(|_| vec![0.0; block]).collect() };
        let mut input = buffers(spec.info.audio_in);
        let mut output = buffers(spec.info.audio_out);
        let mut key = buffers(spec.key_in);
        Self {
            key_ptrs: key.iter_mut().map(|b| b.as_mut_ptr()).collect(),
            key,
            spec,
            plugin,
            in_ptrs: input.iter_mut().map(|b| b.as_mut_ptr()).collect(),
            out_ptrs: output.iter_mut().map(|b| b.as_mut_ptr()).collect(),
            input,
            output,
            lists: Box::new(Lists {
                input: Vec::with_capacity(kit::EVENTS),
                output: Vec::with_capacity(kit::EVENTS),
            }),
            _host: host,
            track,
            _desc: desc,
        }
    }

    fn raw(&self) -> &clap_plugin {
        unsafe { &*self.plugin }
    }

    /// As a host does when the plugin's track is named or renamed.
    pub fn name_track(&self, name: &str) {
        *self.track.borrow_mut() = name.to_string();
        let ext = self.extension::<clap_plugin_track_info>(CLAP_EXT_TRACK_INFO);
        unsafe { ext.changed.expect("changed")(self.plugin) };
    }

    fn extension<T>(&self, id: &CStr) -> &T {
        let get = self.raw().get_extension.expect("get_extension");
        let ext = unsafe { get(self.plugin, id.as_ptr()) } as *const T;
        unsafe { ext.as_ref() }.expect("the extension")
    }

    /// Input channel `ch` for the next block.
    pub fn input(&mut self, ch: usize) -> &mut [f32] {
        &mut self.input[ch]
    }

    /// Sidechain channel `ch` for the next block.
    pub fn key(&mut self, ch: usize) -> &mut [f32] {
        &mut self.key[ch]
    }

    /// Output channel `ch` of the last block.
    pub fn output(&self, ch: usize) -> &[f32] {
        &self.output[ch]
    }

    /// Automates the setting a host knows as `id` in the next block.
    pub fn set(&mut self, id: u32, norm: f64) {
        let Some(p) = self.spec.by_id(id) else {
            return;
        };
        self.lists.input.push(In::Value(clap_event_param_value {
            header: header::<clap_event_param_value>(0, CLAP_EVENT_PARAM_VALUE),
            param_id: id,
            cookie: std::ptr::null_mut(),
            note_id: -1,
            port_index: -1,
            channel: -1,
            key: -1,
            value: p.plain(norm),
        }));
    }

    /// A note or controller for the next block.
    pub fn send(&mut self, midi: Event) {
        self.lists.input.push(In::Midi(clap_event_midi {
            header: header::<clap_event_midi>(midi.offset, CLAP_EVENT_MIDI),
            port_index: 0,
            data: midi.data,
        }));
    }

    /// Plays `frames` of `input` into `output`. No allocation.
    pub fn block(&mut self, frames: usize) {
        self.lists.output.clear();
        let ctx = &mut *self.lists as *mut Lists as *mut c_void;
        let in_events = clap_input_events {
            ctx,
            size: Some(in_size),
            get: Some(in_get),
        };
        let out_events = clap_output_events {
            ctx,
            try_push: Some(out_push),
        };
        let buffer = |ptrs: &mut Vec<*mut f32>| clap_audio_buffer {
            data32: ptrs.as_mut_ptr(),
            data64: std::ptr::null_mut(),
            channel_count: ptrs.len() as u32,
            latency: 0,
            constant_mask: 0,
        };
        // The input, then the sidechain if the plugin has one.
        let audio_in = [buffer(&mut self.in_ptrs), buffer(&mut self.key_ptrs)];
        let given = u32::from(!self.in_ptrs.is_empty()) + u32::from(!self.key_ptrs.is_empty());
        let mut audio_out = buffer(&mut self.out_ptrs);
        let process = clap_process {
            steady_time: -1,
            frames_count: frames as u32,
            transport: std::ptr::null(),
            audio_inputs: audio_in.as_ptr(),
            audio_outputs: &mut audio_out,
            audio_inputs_count: given,
            audio_outputs_count: 1,
            in_events: &in_events,
            out_events: &out_events,
        };
        let run = self.raw().process.expect("process");
        assert_ne!(unsafe { run(self.plugin, &process) }, CLAP_PROCESS_ERROR);
        self.lists.input.clear();
    }

    /// Plays all of `input` (by channel) in blocks of `block` and
    /// gives back everything that came out. Allocates the result.
    pub fn render(&mut self, input: &[Vec<f32>], frames: usize, block: usize) -> Vec<Vec<f32>> {
        let mut out = vec![Vec::with_capacity(frames); self.output.len()];
        let mut start = 0;
        while start < frames {
            let n = (frames - start).min(block);
            let main = self.input.len();
            let all = self.input.iter_mut().chain(self.key.iter_mut());
            for (ch, buf) in all.enumerate() {
                match kit::source(input, main, ch) {
                    Some(src) => buf[..n].copy_from_slice(&src[start..start + n]),
                    None => buf[..n].fill(0.0),
                }
            }
            self.block(n);
            for (ch, buf) in self.output.iter().enumerate() {
                out[ch].extend_from_slice(&buf[..n]);
            }
            start += n;
        }
        out
    }

    /// What the last block sent out.
    pub fn sent(&self) -> &[Out] {
        &self.lists.output
    }

    /// The plain value of the setting a host knows as `id`.
    pub fn value(&self, id: u32) -> Option<f64> {
        let params = self.extension::<clap_plugin_params>(CLAP_EXT_PARAMS);
        let mut out = 0.0;
        unsafe { params.get_value.expect("get_value")(self.plugin, id, &mut out) }.then_some(out)
    }

    /// What the host would save with a project.
    pub fn save(&self) -> Vec<u8> {
        let state = self.extension::<clap_plugin_state>(CLAP_EXT_STATE);
        let mut bytes = Vec::new();
        let stream = clap_ostream {
            ctx: &mut bytes as *mut Vec<u8> as *mut c_void,
            write: Some(write),
        };
        assert!(unsafe { state.save.expect("save")(self.plugin, &stream) });
        bytes
    }

    pub fn restore(&self, bytes: &[u8]) -> bool {
        let state = self.extension::<clap_plugin_state>(CLAP_EXT_STATE);
        let mut held = (bytes.to_vec(), 0usize);
        let stream = clap_istream {
            ctx: &mut held as *mut (Vec<u8>, usize) as *mut c_void,
            read: Some(read),
        };
        unsafe { state.load.expect("load")(self.plugin, &stream) }
    }

    pub fn latency(&self) -> u32 {
        let latency = self.extension::<clap_plugin_latency>(CLAP_EXT_LATENCY);
        unsafe { latency.get.expect("get")(self.plugin) }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        let raw = self.raw();
        unsafe {
            raw.stop_processing.expect("stop")(self.plugin);
            raw.deactivate.expect("deactivate")(self.plugin);
            raw.destroy.expect("destroy")(self.plugin);
        }
    }
}

fn text(buf: &[c_char]) -> String {
    buf.iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u8 as char)
        .collect()
}

/// What every plugin must get right as a CLAP, whatever it does: its
/// parameters are the spec's in their own units, and its state
/// survives a save and a load. Panics with what is wrong.
pub fn conforms<P: Plugin>() {
    let mut host = Host::load::<P>(48_000.0, 256);
    let spec = host.spec;
    let params = host.extension::<clap_plugin_params>(CLAP_EXT_PARAMS);

    // The input, and after it a sidechain that is not the main port.
    let ports = host.extension::<clap_plugin_audio_ports>(CLAP_EXT_AUDIO_PORTS);
    let wanted = u32::from(spec.info.audio_in > 0) + u32::from(spec.key_in > 0);
    assert_eq!(
        unsafe { ports.count.expect("count")(host.plugin, true) },
        wanted
    );
    if spec.key_in > 0 {
        let mut port: clap_audio_port_info = unsafe { std::mem::zeroed() };
        assert!(unsafe { ports.get.expect("get")(host.plugin, 1, true, &mut port) });
        assert_eq!(port.flags & CLAP_AUDIO_PORT_IS_MAIN, 0, "a sidechain");
        assert_eq!(port.channel_count, spec.key_in);
        assert_eq!(port.in_place_pair, clap_sys::id::CLAP_INVALID_ID);
    }

    let count = unsafe { params.count.expect("count")(host.plugin) };
    assert_eq!(count as usize, spec.params.len(), "every setting");
    for (i, p) in spec.params.iter().enumerate() {
        let mut info: clap_param_info = unsafe { std::mem::zeroed() };
        assert!(
            unsafe { params.get_info.expect("get_info")(host.plugin, i as u32, &mut info) },
            "{}",
            p.key
        );
        assert_eq!((info.id, text(&info.name)), (p.id, p.title.to_string()));
        assert_eq!(
            (info.min_value, info.max_value, info.default_value),
            (p.plain(0.0), p.plain(1.0), p.default),
            "{}",
            p.key
        );
        assert_eq!(
            host.value(p.id),
            Some(p.default),
            "{} starts at its default",
            p.key
        );
    }

    // Every setting moved, saved, and read back by a fresh instance.
    let moved: Vec<f64> = spec
        .params
        .iter()
        .enumerate()
        .map(|(i, p)| p.plain(0.15 + 0.7 * (i as f64 * 0.37).fract()))
        .collect();
    for (p, &plain) in spec.params.iter().zip(&moved) {
        host.set(p.id, p.normalized(plain));
    }
    host.block(64);
    let saved = host.save();
    let fresh = Host::load::<P>(48_000.0, 256);
    assert!(fresh.restore(&saved), "a saved state loads");
    assert_eq!(fresh.save(), saved, "and saves the same again");
    for (p, &plain) in spec.params.iter().zip(&moved) {
        let got = fresh.value(p.id).unwrap_or(f64::NAN);
        assert!((got - plain).abs() < 1e-6, "{}: {got} for {plain}", p.key);
    }
    assert!(!fresh.restore(b"not a state"), "junk is refused");
}
