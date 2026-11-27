//! The audio side: buses from the spec, state by id, and a `Runner`
//! built off the audio thread that `process` only ever try-locks, so it
//! never waits. Meters go back to the host as read-only parameters.

use crate::events::{from_event, to_event};
use crate::strings::copy_wstring;
use crate::{class_id, transport};
use kit::{Plugin, Runner, Shared, Spec, state};
use std::slice;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use vst3::Steinberg::Vst::*;
use vst3::Steinberg::*;
use vst3::{Class, ComRef};

// Smallest meter change worth reporting: a tenth of a dB or so.
const METER_STEP: f64 = 0.0005;

pub struct Processor<P: Plugin> {
    spec: Spec,
    shared: Arc<Shared>,
    // Made in setupProcessing; `process` try-locks it.
    runner: Mutex<Option<Runner<P>>>,
    // Speaker arrangements in force, input then output.
    arrangement: [AtomicU64; 2],
    // And the sidechain's, for a plugin that has one.
    key: AtomicU64,
    // Meter values last reported to the host.
    reported: Box<[AtomicU64]>,
    latency: AtomicU32,
    tail: AtomicU32,
}

impl<P: Plugin> Class for Processor<P> {
    type Interfaces = (
        IComponent,
        IAudioProcessor,
        IProcessContextRequirements,
        IConnectionPoint,
    );
}

// When the host connects the controller, it is offered the sound to
// draw: see `connect`.
impl<P: Plugin> Processor<P> {
    // Whether buses of this kind and direction include a sidechain.
    fn keyed(&self, media: MediaType, input: bool) -> bool {
        input && media as MediaTypes == MediaTypes_::kAudio && self.spec.key_in > 0
    }
}

impl<P: Plugin> IConnectionPointTrait for Processor<P> {
    unsafe fn connect(&self, other: *mut IConnectionPoint) -> tresult {
        let Some(other) = (unsafe { ComRef::from_raw(other) }) else {
            return kInvalidArgument;
        };
        crate::connect::offer(&other, self.shared.scope());
        kResultOk
    }

    unsafe fn disconnect(&self, _other: *mut IConnectionPoint) -> tresult {
        kResultOk
    }

    unsafe fn notify(&self, _message: *mut IMessage) -> tresult {
        kResultFalse
    }
}

fn speakers(channels: u32) -> SpeakerArrangement {
    match channels {
        0 => 0,
        1 => SpeakerArr::kMono,
        _ => SpeakerArr::kStereo,
    }
}

// (channels, name) of the one bus of a kind and direction, if any.
fn bus(spec: &Spec, media: MediaTypes, input: bool) -> Option<(i32, &'static str)> {
    let info = spec.info;
    match (media, input) {
        (MediaTypes_::kAudio, true) if info.audio_in > 0 => Some((info.audio_in as i32, "Input")),
        (MediaTypes_::kAudio, false) => Some((info.audio_out as i32, "Output")),
        // Every plugin takes MIDI in, even one that plays no notes: it
        // is how a controller reaches a setting that has learned it.
        (MediaTypes_::kEvent, true) => Some((16, "MIDI In")),
        (MediaTypes_::kEvent, false) if info.midi_out => Some((16, "MIDI Out")),
        _ => None,
    }
}

impl<P: Plugin> Processor<P> {
    pub fn new() -> Self {
        let spec = Spec::of::<P>();
        Self {
            spec,
            shared: {
                let shared = Arc::new(Shared::new(&spec));
                kit::link::start(&shared, spec);
                shared
            },
            runner: Mutex::new(None),
            arrangement: [
                AtomicU64::new(speakers(spec.info.audio_in)),
                AtomicU64::new(speakers(spec.info.audio_out)),
            ],
            key: AtomicU64::new(speakers(spec.key_in)),
            reported: spec.meters.iter().map(|_| AtomicU64::new(0)).collect(),
            latency: AtomicU32::new(0),
            tail: AtomicU32::new(0),
        }
    }

    // The last point of each queue: a block takes one value a setting.
    unsafe fn read_param_changes(&self, changes: *mut IParameterChanges) {
        let Some(changes) = (unsafe { ComRef::from_raw(changes) }) else {
            return;
        };

        for i in 0..unsafe { changes.getParameterCount() } {
            let Some(queue) = (unsafe { ComRef::from_raw(changes.getParameterData(i)) }) else {
                continue;
            };
            let points = unsafe { queue.getPointCount() };
            if points == 0 {
                continue;
            }

            let (mut offset, mut value) = (0, 0.0);
            let id = unsafe { queue.getParameterId() };
            if unsafe { queue.getPoint(points - 1, &mut offset, &mut value) } != kResultTrue {
                continue;
            }
            // A MIDI controller reaches a VST3 as the parameter that
            // stands in for it: see the controller's `IMidiMapping`.
            if let Some(cc) = crate::controller::cc_of(id) {
                self.shared
                    .controller(cc, (value.clamp(0.0, 1.0) * 127.0).round() as u8);
            } else if let Some(index) = self.spec.setting_of(id) {
                self.shared.set(index, value);
            }
        }
    }

    // Sends each meter to the host only when it moved enough to see,
    // and each setting a MIDI controller moved, so the controller and
    // its editor show where it now is.
    unsafe fn report(&self, changes: *mut IParameterChanges, offset: i32, meters: &[f64]) {
        let Some(changes) = (unsafe { ComRef::from_raw(changes) }) else {
            return;
        };

        for (i, row) in self.spec.params.iter().enumerate() {
            if self.shared.take_edit(i) == 0 {
                continue;
            }
            let mut index = 0;
            let queue = unsafe { changes.addParameterData(&row.id, &mut index) };
            if let (Some(queue), Some(value)) =
                (unsafe { ComRef::from_raw(queue) }, self.shared.value(i))
            {
                let mut point = 0;
                unsafe { queue.addPoint(offset, value, &mut point) };
            }
        }

        for ((slot, row), &value) in self.reported.iter().zip(self.spec.meters).zip(meters) {
            let last = f64::from_bits(slot.load(Ordering::Relaxed));
            if (value - last).abs() < METER_STEP {
                continue;
            }

            let mut index = 0;
            let queue = unsafe { changes.addParameterData(&row.id, &mut index) };
            let Some(queue) = (unsafe { ComRef::from_raw(queue) }) else {
                continue;
            };
            let mut point = 0;
            if unsafe { queue.addPoint(offset, value, &mut point) } == kResultOk {
                slot.store(value.to_bits(), Ordering::Relaxed);
            }
        }
    }
}

impl<P: Plugin> Default for Processor<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Plugin> IPluginBaseTrait for Processor<P> {
    unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }

    unsafe fn terminate(&self) -> tresult {
        kResultOk
    }
}

impl<P: Plugin> IComponentTrait for Processor<P> {
    unsafe fn getControllerClassId(&self, id: *mut TUID) -> tresult {
        let Some(id) = (unsafe { id.as_mut() }) else {
            return kInvalidArgument;
        };

        *id = class_id(self.spec.info.vst3_controller);
        kResultOk
    }

    unsafe fn setIoMode(&self, _mode: IoMode) -> tresult {
        kResultOk
    }

    unsafe fn getBusCount(&self, media: MediaType, dir: BusDirection) -> i32 {
        let input = dir as BusDirections == BusDirections_::kInput;
        let main = bus(&self.spec, media as MediaTypes, input).map_or(0, |_| 1);
        main + i32::from(self.keyed(media, input))
    }

    unsafe fn getBusInfo(
        &self,
        media: MediaType,
        dir: BusDirection,
        index: i32,
        info: *mut BusInfo,
    ) -> tresult {
        let input = dir as BusDirections == BusDirections_::kInput;
        // The sidechain: a second audio input, marked as one, which
        // the host leaves off until something is routed to it.
        if let (true, 1, Some(info)) = (self.keyed(media, input), index, unsafe { info.as_mut() }) {
            info.mediaType = media;
            info.direction = dir;
            info.channelCount = self.key.load(Ordering::Relaxed).count_ones() as i32;
            copy_wstring("Sidechain", &mut info.name);
            info.busType = BusTypes_::kAux as BusType;
            info.flags = 0;
            return kResultOk;
        }
        let (Some((channels, name)), Some(info), 0) = (
            bus(&self.spec, media as MediaTypes, input),
            unsafe { info.as_mut() },
            index,
        ) else {
            return kInvalidArgument;
        };

        info.mediaType = media;
        info.direction = dir;
        // Audio buses follow the arrangement in force: one bit a speaker.
        let which = if input { 0 } else { 1 };
        info.channelCount = match media as MediaTypes {
            MediaTypes_::kAudio => {
                self.arrangement[which].load(Ordering::Relaxed).count_ones() as i32
            }
            _ => channels,
        };
        copy_wstring(name, &mut info.name);
        info.busType = BusTypes_::kMain as BusType;
        info.flags = BusInfo_::BusFlags_::kDefaultActive as u32;
        kResultOk
    }

    unsafe fn getRoutingInfo(
        &self,
        _in_info: *mut RoutingInfo,
        _out_info: *mut RoutingInfo,
    ) -> tresult {
        kNotImplemented
    }

    unsafe fn activateBus(
        &self,
        _media: MediaType,
        _dir: BusDirection,
        _index: i32,
        _state: TBool,
    ) -> tresult {
        kResultOk
    }

    unsafe fn setActive(&self, state: TBool) -> tresult {
        // A fresh start each time the host switches it on.
        if state != 0
            && let Ok(mut slot) = self.runner.lock()
            && let Some(runner) = slot.as_mut()
        {
            runner.reset();
        }
        kResultOk
    }

    unsafe fn setState(&self, stream: *mut IBStream) -> tresult {
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
        self.shared.set_all(&values);
        self.shared.set_learnt(&state::learnt(&self.spec, &bytes));
        self.shared.set_links(&state::linked(&self.spec, &bytes));
        kResultOk
    }

    unsafe fn getState(&self, stream: *mut IBStream) -> tresult {
        let Some(stream) = (unsafe { ComRef::from_raw(stream) }) else {
            return kInvalidArgument;
        };

        let mut bytes = state::encode_linked(
            &self.spec,
            &self.shared.values(),
            &self.shared.learnt(),
            &self.shared.links(),
        );
        let mut written = 0;
        unsafe { stream.write(bytes.as_mut_ptr().cast(), bytes.len() as i32, &mut written) }
    }
}

/// Everything left in `stream`, a chunk at a time.
///
/// Hosts disagree about the end of a stream. Some answer a read that
/// could not be filled with success and a short count; others call
/// that a failure, though they have handed over what there was. So
/// what a read returns is kept whatever it answers, and the stream is
/// over when a read brings nothing or fails. Treating that failure as
/// "no state" loses a whole project's settings in the second kind.
///
/// # Safety
/// `stream` must be a live stream.
pub unsafe fn read_all(stream: &ComRef<'_, IBStream>) -> Result<Vec<u8>, tresult> {
    let mut bytes = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let mut read = 0;
        let rc = unsafe { stream.read(buf.as_mut_ptr().cast(), buf.len() as i32, &mut read) };
        let got = read.clamp(0, buf.len() as i32) as usize;
        bytes.extend_from_slice(&buf[..got]);
        if rc != kResultOk || got == 0 {
            return Ok(bytes);
        }
    }
}

impl<P: Plugin> IAudioProcessorTrait for Processor<P> {
    unsafe fn setBusArrangements(
        &self,
        inputs: *mut SpeakerArrangement,
        num_ins: i32,
        outputs: *mut SpeakerArrangement,
        num_outs: i32,
    ) -> tresult {
        let info = self.spec.info;
        let wants_in = i32::from(info.audio_in > 0);
        // A host may leave the sidechain out or name it.
        let with_key = wants_in + i32::from(self.spec.key_in > 0);
        if !(num_ins == wants_in || num_ins == with_key) || num_outs != 1 || outputs.is_null() {
            return kResultFalse;
        }
        if num_ins > 0 && inputs.is_null() {
            return kResultFalse;
        }
        // The sidechain may be mono, as wide as built, or off.
        let key = match num_ins {
            2 => unsafe { *inputs.add(1) },
            _ => self.key.load(Ordering::Relaxed),
        };
        if ![0, SpeakerArr::kMono, speakers(self.spec.key_in)].contains(&key) {
            return kResultFalse;
        }

        // A mono track may feed a stereo input; the output is as built.
        let output = unsafe { *outputs };
        let input = match wants_in {
            1 => unsafe { *inputs },
            _ => 0,
        };
        let input_ok =
            wants_in == 0 || input == SpeakerArr::kMono || input == speakers(info.audio_in);
        if !input_ok || output != speakers(info.audio_out) {
            return kResultFalse;
        }
        self.arrangement[0].store(input, Ordering::Relaxed);
        self.arrangement[1].store(output, Ordering::Relaxed);
        self.key.store(key, Ordering::Relaxed);
        kResultTrue
    }

    unsafe fn getBusArrangement(
        &self,
        dir: BusDirection,
        index: i32,
        arr: *mut SpeakerArrangement,
    ) -> tresult {
        let Some(arr) = (unsafe { arr.as_mut() }) else {
            return kInvalidArgument;
        };
        let input = dir as BusDirections == BusDirections_::kInput;
        if index == 1 && input && self.spec.key_in > 0 {
            *arr = self.key.load(Ordering::Relaxed);
            return kResultOk;
        }
        if index != 0 {
            return kInvalidArgument;
        }

        let which = match dir as BusDirections {
            BusDirections_::kInput => 0,
            _ => 1,
        };
        *arr = self.arrangement[which].load(Ordering::Relaxed);
        kResultOk
    }

    unsafe fn canProcessSampleSize(&self, size: i32) -> tresult {
        match size as SymbolicSampleSizes {
            SymbolicSampleSizes_::kSample32 => kResultTrue,
            _ => kResultFalse,
        }
    }

    unsafe fn getLatencySamples(&self) -> u32 {
        self.latency.load(Ordering::Relaxed)
    }

    unsafe fn setupProcessing(&self, setup: *mut ProcessSetup) -> tresult {
        let Some(setup) = (unsafe { setup.as_ref() }) else {
            return kInvalidArgument;
        };

        let block = setup.maxSamplesPerBlock.max(1) as usize;
        let mut runner = Runner::<P>::new(setup.sampleRate as f32, block);
        runner.watch(self.shared.scope());
        self.latency.store(runner.latency(), Ordering::Relaxed);
        self.tail.store(runner.tail(), Ordering::Relaxed);
        match self.runner.lock() {
            Ok(mut slot) => *slot = Some(runner),
            Err(_) => return kResultFalse,
        }
        kResultOk
    }

    unsafe fn setProcessing(&self, _state: TBool) -> tresult {
        kResultOk
    }

    // Each piece of input is read in full before its output is written:
    // hosts may hand the same buffer in as input and out as output.
    unsafe fn process(&self, data: *mut ProcessData) -> tresult {
        let Some(data) = (unsafe { data.as_mut() }) else {
            return kInvalidArgument;
        };
        unsafe { self.read_param_changes(data.inputParameterChanges) };
        let n = data.numSamples.max(0) as usize;
        let ins = unsafe { channels(data.inputs, data.numInputs) };
        // The sidechain is the second input bus, when the host gives it.
        let key = match data.numInputs > 1 && self.spec.key_in > 0 {
            true => unsafe { channels(data.inputs.add(1), 1) },
            false => &[],
        };
        let main = self.spec.info.audio_in as usize;
        let outs = unsafe { channels(data.outputs, data.numOutputs) };

        let Ok(mut slot) = self.runner.try_lock() else {
            unsafe { silence(outs, n) };
            return kResultOk;
        };
        let Some(runner) = slot.as_mut() else {
            unsafe { silence(outs, n) };
            return kResultOk;
        };

        self.shared.snapshot(&mut runner.params);
        // Notes are only for a plugin that asked for them.
        let notes = unsafe { ComRef::from_raw(data.inputEvents) };
        if let Some(events) = notes.filter(|_| self.spec.info.midi_in) {
            for i in 0..unsafe { events.getEventCount() } {
                let mut ev: Event = unsafe { std::mem::zeroed() };
                if unsafe { events.getEvent(i, &mut ev) } == kResultOk
                    && let Some(midi) = from_event(&ev)
                {
                    runner.push(midi);
                }
            }
        }

        let sink = unsafe { ComRef::from_raw(data.outputEvents) };
        let made = self.spec.info.audio_out as usize;
        runner.run(
            n,
            transport::read(unsafe { data.processContext.as_ref() }),
            |ch, start, into| {
                let src = match ch < main {
                    true => ins.get(ch).or(ins.last()),
                    false => key.get(ch - main).or(key.last()),
                }
                .filter(|p| !p.is_null());
                match src {
                    Some(&p) => into.copy_from_slice(unsafe {
                        slice::from_raw_parts(p.add(start), into.len())
                    }),
                    None => into.fill(0.0),
                }
            },
            |ch, start, from| {
                if let Some(&p) = outs.get(ch).filter(|p| !p.is_null()) {
                    unsafe { slice::from_raw_parts_mut(p.add(start), from.len()) }
                        .copy_from_slice(from);
                }
            },
            |midi| {
                if let (Some(sink), Some(mut ev)) = (sink.as_ref(), to_event(midi)) {
                    unsafe { sink.addEvent(&mut ev) };
                }
            },
        );
        // Channels the host has that the plugin does not make.
        unsafe { silence(outs.get(made..).unwrap_or(&[]), n) };
        if let Some(bus) = unsafe { data.outputs.as_mut() }.filter(|_| data.numOutputs > 0) {
            bus.silenceFlags = 0;
        }

        for (i, &value) in runner.meters().iter().enumerate() {
            self.shared.set_meter(i, value);
        }
        let last = n.saturating_sub(1) as i32;
        unsafe { self.report(data.outputParameterChanges, last, runner.meters()) };
        kResultOk
    }

    unsafe fn getTailSamples(&self) -> u32 {
        self.tail.load(Ordering::Relaxed)
    }
}

impl<P: Plugin> IProcessContextRequirementsTrait for Processor<P> {
    unsafe fn getProcessContextRequirements(&self) -> u32 {
        transport::NEEDS
    }
}

// The first bus's channel pointers, or none when the host sent no bus.
unsafe fn channels<'a>(buses: *mut AudioBusBuffers, count: i32) -> &'a [*mut f32] {
    if count < 1 || buses.is_null() {
        return &[];
    }

    let bus = unsafe { &*buses };
    let buffers = unsafe { bus.__field0.channelBuffers32 };
    if buffers.is_null() || bus.numChannels < 1 {
        return &[];
    }
    unsafe { slice::from_raw_parts(buffers, bus.numChannels as usize) }
}

unsafe fn silence(outs: &[*mut f32], n: usize) {
    outs.iter()
        .filter(|p| !p.is_null())
        .for_each(|&p| unsafe { slice::from_raw_parts_mut(p, n) }.fill(0.0));
}
