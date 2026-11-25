//! The plugin instance. `Core` is everything about it that does not
//! depend on the plugin's type, and sits first in `Instance<P>` so the
//! extensions can reach it through the pointer the host passes back
//! without knowing `P`. Only making, activating, running and freeing
//! the instance are generic.

use crate::{params, ports, transport};
use clap_sys::events::*;
use clap_sys::ext::latency::{CLAP_EXT_LATENCY, clap_host_latency, clap_plugin_latency};
use clap_sys::ext::params::{CLAP_EXT_PARAMS, CLAP_PARAM_RESCAN_VALUES, clap_host_params};
use clap_sys::host::clap_host;
use clap_sys::plugin::{clap_plugin, clap_plugin_descriptor};
use clap_sys::process::{
    CLAP_PROCESS_CONTINUE, CLAP_PROCESS_ERROR, clap_process, clap_process_status,
};
use kit::{BEGIN, END, Event, Param, Plugin, Runner, SET, Shared, Spec};
use std::cell::UnsafeCell;
use std::ffi::{CStr, c_char, c_void};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

#[repr(C)]
pub struct Core {
    raw: clap_plugin,
    host: *const clap_host,
    pub spec: Spec,
    /// Settings and meters, readable from any thread.
    pub shared: Arc<Shared>,
    // The latency activate last told the host of.
    latency: AtomicU32,
    #[cfg(windows)]
    pub gui: std::cell::RefCell<Option<crate::gui::Gui>>,
}

#[repr(C)]
struct Instance<P: Plugin> {
    core: Core,
    // The audio thread's alone, between activate and deactivate.
    runner: UnsafeCell<Option<Runner<P>>>,
}

impl Core {
    /// The instance behind a pointer the host passes back.
    ///
    /// # Safety
    /// `plugin` must be one `create` returned and not yet destroyed.
    pub unsafe fn from<'a>(plugin: *const clap_plugin) -> Option<&'a Self> {
        let plugin = unsafe { plugin.as_ref()? };
        unsafe { (plugin.plugin_data as *const Self).as_ref() }
    }

    /// The setting the host knows as `id`.
    pub fn setting(&self, id: u32) -> Option<&'static Param> {
        self.spec.setting_of(id).map(|i| &self.spec.params[i])
    }

    fn host_extension<T>(&self, id: &CStr) -> Option<(&clap_host, &T)> {
        let host = unsafe { self.host.as_ref() }?;
        let get = host.get_extension?;
        let ext = unsafe { get(host, id.as_ptr()) } as *const T;
        Some((host, unsafe { ext.as_ref() }?))
    }

    /// Asks the host to call `flush` (or `process`) soon, to hear about
    /// the editor's gestures.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn request_flush(&self) {
        if let Some((host, ext)) = self.host_extension::<clap_host_params>(CLAP_EXT_PARAMS)
            && let Some(flush) = ext.request_flush
        {
            unsafe { flush(host) };
        }
    }

    /// Tells the host every value may have changed, as after a load.
    pub fn rescan_values(&self) {
        if let Some((host, ext)) = self.host_extension::<clap_host_params>(CLAP_EXT_PARAMS)
            && let Some(rescan) = ext.rescan
        {
            unsafe { rescan(host, CLAP_PARAM_RESCAN_VALUES) };
        }
    }
}

impl<P: Plugin> Instance<P> {
    unsafe fn from<'a>(plugin: *const clap_plugin) -> Option<&'a Self> {
        let plugin = unsafe { plugin.as_ref()? };
        unsafe { (plugin.plugin_data as *const Self).as_ref() }
    }
}

/// A new instance for `host`, as the factory's `create_plugin` gives.
pub fn create<P: Plugin>(
    desc: *const clap_plugin_descriptor,
    host: *const clap_host,
) -> *const clap_plugin {
    let spec = Spec::of::<P>();
    let instance = Box::new(Instance::<P> {
        core: Core {
            raw: clap_plugin {
                desc,
                plugin_data: std::ptr::null_mut(),
                init: Some(init),
                destroy: Some(destroy::<P>),
                activate: Some(activate::<P>),
                deactivate: Some(deactivate::<P>),
                start_processing: Some(start_processing),
                stop_processing: Some(stop_processing),
                reset: Some(reset::<P>),
                process: Some(process::<P>),
                get_extension: Some(get_extension),
                on_main_thread: Some(on_main_thread),
            },
            host,
            spec,
            shared: Arc::new(Shared::new(&spec)),
            latency: AtomicU32::new(0),
            #[cfg(windows)]
            gui: std::cell::RefCell::new(None),
        },
        runner: UnsafeCell::new(None),
    });
    let raw = Box::into_raw(instance);
    unsafe {
        (*raw).core.raw.plugin_data = raw as *mut c_void;
        &(*raw).core.raw
    }
}

unsafe extern "C" fn init(_: *const clap_plugin) -> bool {
    true
}

unsafe extern "C" fn destroy<P: Plugin>(plugin: *const clap_plugin) {
    let Some(this) = (unsafe { Instance::<P>::from(plugin) }) else {
        return;
    };

    #[cfg(windows)]
    this.core.gui.borrow_mut().take();
    drop(unsafe { Box::from_raw(this as *const Instance<P> as *mut Instance<P>) });
}

unsafe extern "C" fn activate<P: Plugin>(
    plugin: *const clap_plugin,
    rate: f64,
    _min: u32,
    max: u32,
) -> bool {
    let Some(this) = (unsafe { Instance::<P>::from(plugin) }) else {
        return false;
    };

    let mut runner = Runner::<P>::new(rate as f32, (max as usize).max(1));
    runner.watch(this.core.shared.scope());
    // The latency may change only here; the host hears of it at once.
    let latency = runner.latency();
    unsafe { *this.runner.get() = Some(runner) };
    let was = this.core.latency.swap(latency, Ordering::AcqRel);
    if was != latency
        && let Some((host, ext)) = this
            .core
            .host_extension::<clap_host_latency>(CLAP_EXT_LATENCY)
        && let Some(changed) = ext.changed
    {
        unsafe { changed(host) };
    }
    true
}

unsafe extern "C" fn deactivate<P: Plugin>(plugin: *const clap_plugin) {
    if let Some(this) = unsafe { Instance::<P>::from(plugin) } {
        unsafe { *this.runner.get() = None };
    }
}

unsafe extern "C" fn start_processing(_: *const clap_plugin) -> bool {
    true
}

unsafe extern "C" fn stop_processing(_: *const clap_plugin) {}

unsafe extern "C" fn reset<P: Plugin>(plugin: *const clap_plugin) {
    if let Some(this) = unsafe { Instance::<P>::from(plugin) }
        && let Some(runner) = unsafe { (*this.runner.get()).as_mut() }
    {
        runner.reset();
    }
}

unsafe extern "C" fn on_main_thread(_: *const clap_plugin) {}

unsafe extern "C" fn get_extension(_: *const clap_plugin, id: *const c_char) -> *const c_void {
    if id.is_null() {
        return std::ptr::null();
    }

    let id = unsafe { CStr::from_ptr(id) };
    params::extension(id)
        .or_else(|| ports::extension(id))
        .or_else(|| (id == CLAP_EXT_LATENCY).then_some(&LATENCY as *const _ as *const c_void))
        .or_else(|| extension_gui(id))
        .unwrap_or(std::ptr::null())
}

#[cfg(windows)]
fn extension_gui(id: &CStr) -> Option<*const c_void> {
    crate::gui::extension(id)
}

#[cfg(not(windows))]
fn extension_gui(_: &CStr) -> Option<*const c_void> {
    None
}

static LATENCY: clap_plugin_latency = clap_plugin_latency { get: Some(latency) };

unsafe extern "C" fn latency(plugin: *const clap_plugin) -> u32 {
    unsafe { Core::from(plugin) }.map_or(0, |core| core.latency.load(Ordering::Relaxed))
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

fn push<T>(out: *const clap_output_events, event: &T) -> bool {
    let Some(out) = (unsafe { out.as_ref() }) else {
        return false;
    };
    let Some(try_push) = out.try_push else {
        return false;
    };

    unsafe { try_push(out, event as *const T as *const clap_event_header) }
}

// The host's events for a block, each as the header it starts with.
fn headers<'a>(in_events: *const clap_input_events) -> impl Iterator<Item = &'a clap_event_header> {
    let events = unsafe { in_events.as_ref() };
    let (size, get) = events.map_or((None, None), |e| (e.size, e.get));
    let count = match (events, size) {
        (Some(e), Some(size)) => unsafe { size(e) },
        _ => 0,
    };
    (0..count)
        .filter_map(move |i| unsafe { get?(events?, i).as_ref() })
        .filter(|head| head.space_id == CLAP_CORE_EVENT_SPACE_ID)
}

/// A keyboard's notes and messages from the host's events, as MIDI 1.0
/// bytes at their sample offsets: CLAP notes and raw MIDI.
fn keys<'a>(in_events: *const clap_input_events) -> impl Iterator<Item = Event> + 'a {
    headers(in_events).filter_map(|head| {
        let offset = head.time;
        let note = |status: u8| {
            let n = unsafe { &*(head as *const clap_event_header as *const clap_event_note) };
            let velocity = (n.velocity.clamp(0.0, 1.0) * 127.0).round() as u8;
            let velocity = if status == 0x90 { velocity.max(1) } else { 0 };
            let data = [
                status | (n.channel.max(0) as u8 & 0x0F),
                n.key.clamp(0, 127) as u8,
                velocity,
            ];
            Some(Event { offset, data })
        };
        match head.type_ {
            CLAP_EVENT_NOTE_ON => note(0x90),
            CLAP_EVENT_NOTE_OFF | CLAP_EVENT_NOTE_CHOKE => note(0x80),
            CLAP_EVENT_MIDI => {
                let m = unsafe { &*(head as *const clap_event_header as *const clap_event_midi) };
                Some(Event {
                    offset,
                    data: m.data,
                })
            }
            _ => None,
        }
    })
}

/// Host automation in, the editor's gestures out: what both `process`
/// and `flush` do with parameters.
pub fn exchange(core: &Core, in_events: *const clap_input_events, out: *const clap_output_events) {
    for head in headers(in_events).filter(|h| h.type_ == CLAP_EVENT_PARAM_VALUE) {
        let event =
            unsafe { &*(head as *const clap_event_header as *const clap_event_param_value) };
        if let Some(index) = core.spec.setting_of(event.param_id) {
            let norm = core.spec.params[index].normalized(event.value);
            core.shared.set(index, norm);
        }
    }

    for (index, p) in core.spec.params.iter().enumerate() {
        let what = core.shared.take_edit(index);
        if what == 0 {
            continue;
        }

        let gesture = |kind: u16| clap_event_param_gesture {
            header: header::<clap_event_param_gesture>(0, kind),
            param_id: p.id,
        };
        if what & BEGIN != 0 {
            push(out, &gesture(CLAP_EVENT_PARAM_GESTURE_BEGIN));
        }
        if what & SET != 0
            && let Some(norm) = core.shared.value(index)
        {
            let set = clap_event_param_value {
                header: header::<clap_event_param_value>(0, CLAP_EVENT_PARAM_VALUE),
                param_id: p.id,
                cookie: std::ptr::null_mut(),
                note_id: -1,
                port_index: -1,
                channel: -1,
                key: -1,
                value: p.plain(norm),
            };
            push(out, &set);
        }
        if what & END != 0 {
            push(out, &gesture(CLAP_EVENT_PARAM_GESTURE_END));
        }
    }
}

// The first port's channel pointers, or none when there is no port.
unsafe fn channels<'a>(
    buffers: *const clap_sys::audio_buffer::clap_audio_buffer,
    count: u32,
) -> &'a [*mut f32] {
    let Some(port) = (unsafe { buffers.as_ref() }).filter(|_| count > 0) else {
        return &[];
    };
    if port.data32.is_null() || port.channel_count == 0 {
        return &[];
    }
    unsafe { std::slice::from_raw_parts(port.data32, port.channel_count as usize) }
}

// Each piece of input is read in full before its output is written:
// the host may hand the same buffers in and out.
unsafe extern "C" fn process<P: Plugin>(
    plugin: *const clap_plugin,
    process: *const clap_process,
) -> clap_process_status {
    let (Some(this), Some(process)) = (unsafe { Instance::<P>::from(plugin) }, unsafe {
        process.as_ref()
    }) else {
        return CLAP_PROCESS_ERROR;
    };
    let Some(runner) = (unsafe { (*this.runner.get()).as_mut() }) else {
        return CLAP_PROCESS_ERROR;
    };

    let core = &this.core;
    // A controller moves the setting that has learned it, before the
    // host is told what moved and the block reads its settings.
    for midi in keys(process.in_events).filter(|m| m.data[0] & 0xF0 == 0xB0) {
        core.shared.controller(midi.data[1], midi.data[2]);
    }
    exchange(core, process.in_events, process.out_events);
    core.shared.snapshot(&mut runner.params);
    // Notes are only for a plugin that asked for them.
    for midi in keys(process.in_events).filter(|_| core.spec.info.midi_in) {
        runner.push(midi);
    }

    let n = process.frames_count as usize;
    let ins = unsafe { channels(process.audio_inputs, process.audio_inputs_count) };
    let outs = unsafe { channels(process.audio_outputs, process.audio_outputs_count) };
    let made = core.spec.info.audio_out as usize;
    runner.run(
        n,
        transport::read(unsafe { process.transport.as_ref() }, runner.rate()),
        |ch, start, into| {
            let src = ins.get(ch).or(ins.last()).filter(|p| !p.is_null());
            match src {
                Some(&p) => into.copy_from_slice(unsafe {
                    std::slice::from_raw_parts(p.add(start), into.len())
                }),
                None => into.fill(0.0),
            }
        },
        |ch, start, from| {
            if let Some(&p) = outs.get(ch).filter(|p| !p.is_null()) {
                unsafe { std::slice::from_raw_parts_mut(p.add(start), from.len()) }
                    .copy_from_slice(from);
            }
        },
        |midi| {
            let event = clap_event_midi {
                header: header::<clap_event_midi>(midi.offset, CLAP_EVENT_MIDI),
                port_index: 0,
                data: midi.data,
            };
            push(process.out_events, &event);
        },
    );
    // Channels the host has that the plugin does not make.
    for &p in outs
        .get(made..)
        .unwrap_or(&[])
        .iter()
        .filter(|p| !p.is_null())
    {
        unsafe { std::slice::from_raw_parts_mut(p, n) }.fill(0.0);
    }

    for (i, &value) in runner.meters().iter().enumerate() {
        core.shared.set_meter(i, value);
    }
    CLAP_PROCESS_CONTINUE
}
