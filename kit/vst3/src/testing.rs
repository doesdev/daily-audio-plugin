//! A host for tests: loads a plugin through its factory the way a DAW
//! does and plays it a block at a time. Everything a block needs is
//! made up front, so `block` itself never allocates and an allocation
//! counted across it is the plugin's.

use crate::{Factory, class_id};
use kit::{Event as Midi, Plugin, Spec};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use vst3::Steinberg::Vst::ChannelContext::IInfoListenerTrait;
use vst3::Steinberg::Vst::*;
use vst3::Steinberg::*;
use vst3::{Class, ComPtr, ComWrapper, Interface};

const QUEUES: usize = 64;

// One parameter's change for a block: an id and a single point.
#[derive(Default)]
struct Queue {
    id: Cell<ParamID>,
    point: Cell<Option<f64>>,
}

impl Class for Queue {
    type Interfaces = (IParamValueQueue,);
}

impl IParamValueQueueTrait for Queue {
    unsafe fn getParameterId(&self) -> ParamID {
        self.id.get()
    }

    unsafe fn getPointCount(&self) -> i32 {
        i32::from(self.point.get().is_some())
    }

    unsafe fn getPoint(&self, index: i32, offset: *mut i32, value: *mut ParamValue) -> tresult {
        let (0, Some(point)) = (index, self.point.get()) else {
            return kResultFalse;
        };
        unsafe {
            *offset = 0;
            *value = point;
        }
        kResultTrue
    }

    unsafe fn addPoint(&self, _offset: i32, value: ParamValue, index: *mut i32) -> tresult {
        self.point.set(Some(value));
        if !index.is_null() {
            unsafe { *index = 0 };
        }
        kResultOk
    }
}

// The changes of a block, in either direction: a pool of queues and
// how many are in use.
struct Changes {
    queues: Vec<(ComWrapper<Queue>, ComPtr<IParamValueQueue>)>,
    used: Cell<usize>,
}

impl Changes {
    fn new() -> Self {
        let queues = (0..QUEUES)
            .map(|_| {
                let queue = ComWrapper::new(Queue::default());
                let ptr = queue.to_com_ptr().expect("queue");
                (queue, ptr)
            })
            .collect();
        Self {
            queues,
            used: Cell::new(0),
        }
    }

    fn take(&self, id: ParamID) -> Option<&(ComWrapper<Queue>, ComPtr<IParamValueQueue>)> {
        let slot = self.queues.get(self.used.get())?;
        slot.0.id.set(id);
        slot.0.point.set(None);
        self.used.set(self.used.get() + 1);
        Some(slot)
    }
}

impl Class for Changes {
    type Interfaces = (IParameterChanges,);
}

impl IParameterChangesTrait for Changes {
    unsafe fn getParameterCount(&self) -> i32 {
        self.used.get() as i32
    }

    unsafe fn getParameterData(&self, index: i32) -> *mut IParamValueQueue {
        let used = &self.queues[..self.used.get()];
        used.get(index as usize)
            .map_or(std::ptr::null_mut(), |q| q.1.as_ptr())
    }

    unsafe fn addParameterData(
        &self,
        id: *const ParamID,
        index: *mut i32,
    ) -> *mut IParamValueQueue {
        let Some(id) = (unsafe { id.as_ref() }) else {
            return std::ptr::null_mut();
        };
        let at = self.used.get();
        let Some(slot) = self.take(*id) else {
            return std::ptr::null_mut();
        };
        if !index.is_null() {
            unsafe { *index = at as i32 };
        }
        slot.1.as_ptr()
    }
}

// Notes for a block, in or out, in a list sized up front.
struct Notes(RefCell<Vec<Event>>);

impl Class for Notes {
    type Interfaces = (IEventList,);
}

impl IEventListTrait for Notes {
    unsafe fn getEventCount(&self) -> i32 {
        self.0.borrow().len() as i32
    }

    unsafe fn getEvent(&self, index: i32, e: *mut Event) -> tresult {
        match (self.0.borrow().get(index as usize), unsafe { e.as_mut() }) {
            (Some(ev), Some(out)) => {
                *out = *ev;
                kResultOk
            }
            _ => kResultFalse,
        }
    }

    unsafe fn addEvent(&self, e: *mut Event) -> tresult {
        let mut list = self.0.borrow_mut();
        match unsafe { e.as_ref() } {
            Some(ev) if list.len() < list.capacity() => {
                list.push(*ev);
                kResultOk
            }
            _ => kResultFalse,
        }
    }
}

// A stream over bytes in memory, for saved state.
struct Stream(RefCell<(Vec<u8>, usize)>);

impl Class for Stream {
    type Interfaces = (IBStream,);
}

impl IBStreamTrait for Stream {
    unsafe fn read(&self, buffer: *mut c_void, n: i32, read: *mut i32) -> tresult {
        let mut held = self.0.borrow_mut();
        let (bytes, at) = &mut *held;
        let take = (n.max(0) as usize).min(bytes.len() - *at);
        unsafe { std::ptr::copy_nonoverlapping(bytes[*at..].as_ptr(), buffer.cast(), take) };
        *at += take;
        if !read.is_null() {
            unsafe { *read = take as i32 };
        }
        // As the stricter hosts do: a read that could not be filled
        // is a failure, though what there was has been handed over.
        match take == n.max(0) as usize {
            true => kResultOk,
            false => kResultFalse,
        }
    }

    unsafe fn write(&self, buffer: *mut c_void, n: i32, written: *mut i32) -> tresult {
        let from = unsafe { std::slice::from_raw_parts(buffer as *const u8, n.max(0) as usize) };
        self.0.borrow_mut().0.extend_from_slice(from);
        if !written.is_null() {
            unsafe { *written = n.max(0) };
        }
        kResultOk
    }

    unsafe fn seek(&self, _pos: i64, _mode: i32, _result: *mut i64) -> tresult {
        kNotImplemented
    }

    unsafe fn tell(&self, _pos: *mut i64) -> tresult {
        kNotImplemented
    }
}

/// A plugin loaded through its factory, set up and switched on.
pub struct Host {
    pub spec: Spec,
    component: ComPtr<IComponent>,
    proc: ComPtr<IAudioProcessor>,
    pub controller: ComPtr<IEditController>,
    // The next block's input and the last block's output, by channel.
    // Never resized: the pointers below are into them.
    input: Vec<Vec<f32>>,
    output: Vec<Vec<f32>>,
    in_ptrs: Vec<*mut f32>,
    out_ptrs: Vec<*mut f32>,
    key: Vec<Vec<f32>>,
    key_ptrs: Vec<*mut f32>,
    changes_in: (ComWrapper<Changes>, ComPtr<IParameterChanges>),
    changes_out: (ComWrapper<Changes>, ComPtr<IParameterChanges>),
    notes_in: (ComWrapper<Notes>, ComPtr<IEventList>),
    notes_out: (ComWrapper<Notes>, ComPtr<IEventList>),
    context: ProcessContext,
}

fn create<I: Interface>(factory: &ComPtr<IPluginFactory>, class: [u32; 4]) -> ComPtr<I> {
    let cid = class_id(class);
    let mut obj = std::ptr::null_mut();
    let rc = unsafe {
        factory.createInstance(cid.as_ptr().cast(), FUnknown::IID.as_ptr().cast(), &mut obj)
    };
    assert_eq!(rc, kResultOk, "the factory makes the class");
    let unknown = unsafe { ComPtr::from_raw(obj as *mut FUnknown) }.expect("an object");
    unknown.cast::<I>().expect("the interface")
}

// A channel's attributes as a host hands them over: its name, and
// nothing else asked for.
struct Channel(Vec<TChar>);

impl Class for Channel {
    type Interfaces = (IAttributeList,);
}

impl IAttributeListTrait for Channel {
    unsafe fn setInt(&self, _id: IAttributeList_::AttrID, _value: int64) -> tresult {
        kResultFalse
    }

    unsafe fn getInt(&self, _id: IAttributeList_::AttrID, _value: *mut int64) -> tresult {
        kResultFalse
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
        id: IAttributeList_::AttrID,
        string: *mut TChar,
        size: uint32,
    ) -> tresult {
        let asked = unsafe { std::ffi::CStr::from_ptr(id) };
        let name = unsafe { std::ffi::CStr::from_ptr(ChannelContext::kChannelNameKey) };
        let room = size as usize / size_of::<TChar>();
        if asked != name || string.is_null() || room == 0 {
            return kResultFalse;
        }
        let take = self.0.len().min(room - 1);
        unsafe {
            std::ptr::copy_nonoverlapping(self.0.as_ptr(), string, take);
            *string.add(take) = 0;
        }
        kResultOk
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

impl Host {
    /// As a host does when the plugin's channel is named or renamed.
    /// Whether the plugin listens for it at all.
    pub fn name_track(&self, name: &str) -> bool {
        let Some(listener) = self.controller.cast::<ChannelContext::IInfoListener>() else {
            return false;
        };
        let channel = ComWrapper::new(Channel(name.encode_utf16().collect()));
        let list = channel.to_com_ptr::<IAttributeList>().expect("a list");
        unsafe { listener.setChannelContextInfos(list.as_ptr()) == kResultOk }
    }

    /// Loads plugin `P` at `rate`, promised blocks of at most `block`.
    pub fn load<P: Plugin>(rate: f32, block: usize) -> Self {
        Self::load_promised::<P>(rate, block, block)
    }

    /// Loads plugin `P` promised blocks of at most `promised`, with
    /// room to hand it up to `block` all the same, as some hosts do.
    pub fn load_promised<P: Plugin>(rate: f32, promised: usize, block: usize) -> Self {
        let spec = Spec::of::<P>();
        let factory = ComWrapper::new(Factory::<P>::default())
            .to_com_ptr::<IPluginFactory>()
            .expect("factory");
        let component: ComPtr<IComponent> = create(&factory, spec.info.vst3_processor);
        let controller: ComPtr<IEditController> = create(&factory, spec.info.vst3_controller);
        let proc = component.cast::<IAudioProcessor>().expect("a processor");

        let mut setup = ProcessSetup {
            processMode: ProcessModes_::kRealtime as i32,
            symbolicSampleSize: SymbolicSampleSizes_::kSample32 as i32,
            maxSamplesPerBlock: promised as i32,
            sampleRate: rate as f64,
        };
        unsafe {
            assert_eq!(component.initialize(std::ptr::null_mut()), kResultOk);
            assert_eq!(controller.initialize(std::ptr::null_mut()), kResultOk);
            // As a host does: each half is introduced to the other.
            let ends = (
                component.cast::<IConnectionPoint>(),
                controller.cast::<IConnectionPoint>(),
            );
            if let (Some(from), Some(to)) = ends {
                assert_eq!(from.connect(to.as_ptr()), kResultOk);
                assert_eq!(to.connect(from.as_ptr()), kResultOk);
            }
            assert_eq!(proc.setupProcessing(&mut setup), kResultOk);
            assert_eq!(component.setActive(1), kResultOk);
            assert_eq!(proc.setProcessing(1), kResultOk);
        }

        let buffers = |n: u32| -> Vec<Vec<f32>> { (0..n).map(|_| vec![0.0; block]).collect() };
        let mut input = buffers(spec.info.audio_in);
        let mut output = buffers(spec.info.audio_out);
        let mut key = buffers(spec.key_in);
        let key_ptrs = key.iter_mut().map(|b| b.as_mut_ptr()).collect();
        let in_ptrs = input.iter_mut().map(|b| b.as_mut_ptr()).collect();
        let out_ptrs = output.iter_mut().map(|b| b.as_mut_ptr()).collect();
        let changes = || {
            let changes = ComWrapper::new(Changes::new());
            let ptr = changes.to_com_ptr().expect("changes");
            (changes, ptr)
        };
        let notes = || {
            let notes = ComWrapper::new(Notes(RefCell::new(Vec::with_capacity(kit::EVENTS))));
            let ptr = notes.to_com_ptr().expect("notes");
            (notes, ptr)
        };
        let mut context: ProcessContext = unsafe { std::mem::zeroed() };
        context.sampleRate = rate as f64;
        Self {
            spec,
            component,
            proc,
            controller,
            input,
            output,
            key,
            key_ptrs,
            in_ptrs,
            out_ptrs,
            changes_in: changes(),
            changes_out: changes(),
            notes_in: notes(),
            notes_out: notes(),
            context,
        }
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
    pub fn set(&self, id: u32, norm: f64) {
        let queue = self.changes_in.0.take(id).expect("room for a change");
        queue.0.point.set(Some(norm));
    }

    /// A note or controller for the next block.
    pub fn send(&self, midi: Midi) {
        if let Some(ev) = crate::events::to_event(midi) {
            self.notes_in.0.0.borrow_mut().push(ev);
        }
    }

    /// Plays `frames` of `input` into `output`. No allocation.
    pub fn block(&mut self, frames: usize) {
        let bus = |ptrs: &mut Vec<*mut f32>| AudioBusBuffers {
            numChannels: ptrs.len() as i32,
            silenceFlags: 0,
            __field0: AudioBusBuffers__type0 {
                channelBuffers32: ptrs.as_mut_ptr(),
            },
        };
        // The input, then the sidechain if the plugin has one.
        let mut inputs = [bus(&mut self.in_ptrs), bus(&mut self.key_ptrs)];
        let given = i32::from(!self.in_ptrs.is_empty()) + i32::from(!self.key_ptrs.is_empty());
        let mut output = AudioBusBuffers {
            numChannels: self.out_ptrs.len() as i32,
            silenceFlags: 0,
            __field0: AudioBusBuffers__type0 {
                channelBuffers32: self.out_ptrs.as_mut_ptr(),
            },
        };
        self.changes_out.0.used.set(0);
        self.notes_out.0.0.borrow_mut().clear();
        let mut data = ProcessData {
            processMode: ProcessModes_::kRealtime as i32,
            symbolicSampleSize: SymbolicSampleSizes_::kSample32 as i32,
            numSamples: frames as i32,
            numInputs: given,
            numOutputs: 1,
            inputs: inputs.as_mut_ptr(),
            outputs: &mut output,
            inputParameterChanges: self.changes_in.1.as_ptr(),
            outputParameterChanges: self.changes_out.1.as_ptr(),
            inputEvents: self.notes_in.1.as_ptr(),
            outputEvents: self.notes_out.1.as_ptr(),
            processContext: &mut self.context,
        };
        assert_eq!(unsafe { self.proc.process(&mut data) }, kResultOk);
        self.changes_in.0.used.set(0);
        self.notes_in.0.0.borrow_mut().clear();
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

    /// The meters the last block reported, as (host id, value).
    pub fn reported(&self) -> Vec<(u32, f64)> {
        let changes = &self.changes_out.0;
        changes.queues[..changes.used.get()]
            .iter()
            .filter_map(|(q, _)| Some((q.id.get(), q.point.get()?)))
            .collect()
    }

    /// The MIDI the last block sent, as VST3 event types and offsets.
    pub fn sent(&self) -> Vec<(u16, i32)> {
        let notes = self.notes_out.0.0.borrow();
        notes.iter().map(|e| (e.r#type, e.sampleOffset)).collect()
    }

    /// What the host would save with a project.
    pub fn save(&self) -> Vec<u8> {
        let stream = ComWrapper::new(Stream(RefCell::default()));
        let ptr = stream.to_com_ptr::<IBStream>().expect("stream");
        assert_eq!(unsafe { self.component.getState(ptr.as_ptr()) }, kResultOk);
        stream.0.borrow().0.clone()
    }

    /// Loads saved state into both sides, as a host does.
    pub fn restore(&self, bytes: &[u8]) -> bool {
        let stream = |bytes: &[u8]| {
            ComWrapper::new(Stream(RefCell::new((bytes.to_vec(), 0))))
                .to_com_ptr::<IBStream>()
                .expect("stream")
        };
        let loaded = unsafe { self.component.setState(stream(bytes).as_ptr()) } == kResultOk;
        let mirrored =
            unsafe { self.controller.setComponentState(stream(bytes).as_ptr()) } == kResultOk;
        loaded && mirrored
    }

    pub fn latency(&self) -> u32 {
        unsafe { self.proc.getLatencySamples() }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        unsafe {
            self.proc.setProcessing(0);
            self.component.setActive(0);
            self.controller.terminate();
            self.component.terminate();
        }
    }
}

/// What every plugin must get right as a VST3, whatever it does: the
/// factory describes it, its parameters are the spec's, and its state
/// survives a save and a load. Panics with what is wrong.
pub fn conforms<P: Plugin>() {
    let mut host = Host::load::<P>(48_000.0, 256);
    let spec = host.spec;

    // The input, and after it a sidechain marked as one.
    let audio = MediaTypes_::kAudio as MediaType;
    let into = BusDirections_::kInput as BusDirection;
    let wanted = i32::from(spec.info.audio_in > 0) + i32::from(spec.key_in > 0);
    assert_eq!(unsafe { host.component.getBusCount(audio, into) }, wanted);
    if spec.key_in > 0 {
        let mut bus: BusInfo = unsafe { std::mem::zeroed() };
        assert_eq!(
            unsafe { host.component.getBusInfo(audio, into, 1, &mut bus) },
            kResultOk
        );
        assert_eq!(bus.busType, BusTypes_::kAux as BusType, "a sidechain");
        assert_eq!(bus.channelCount as u32, spec.key_in);
        assert_eq!(bus.flags, 0, "off until the host routes something to it");
    }

    let count = unsafe { host.controller.getParameterCount() };
    assert_eq!(
        count as usize,
        spec.total() + 128,
        "every setting and meter, then a stand-in for each MIDI controller"
    );
    // The stand-ins: hidden, never automated, and where the mapping
    // sends each controller.
    let mapping = host
        .controller
        .cast::<IMidiMapping>()
        .expect("a MIDI mapping");
    for cc in [0_u32, 74, 127] {
        let mut info: ParameterInfo = unsafe { std::mem::zeroed() };
        let at = (spec.total() as u32 + cc) as i32;
        assert_eq!(
            unsafe { host.controller.getParameterInfo(at, &mut info) },
            kResultOk
        );
        assert_eq!(info.id, crate::controller::CC_PARAMS + cc);
        use ParameterInfo_::ParameterFlags_::{kCanAutomate, kIsHidden};
        assert_ne!(info.flags & kIsHidden as i32, 0, "CC {cc} is hidden");
        assert_eq!(
            info.flags & kCanAutomate as i32,
            0,
            "CC {cc} is not automated"
        );
        let mut id = 0;
        let rc = unsafe { mapping.getMidiControllerAssignment(0, 0, cc as i16, &mut id) };
        assert_eq!((rc, id), (kResultTrue, info.id), "CC {cc} is mapped");
        assert!(
            spec.index_of(info.id).is_none(),
            "CC {cc} is no setting's id"
        );
    }
    let mut id = 0;
    let beyond = unsafe { mapping.getMidiControllerAssignment(0, 0, 128, &mut id) };
    assert_eq!(
        beyond, kResultFalse,
        "pitch bend and the rest are not mapped"
    );
    for (i, p) in spec.rows().enumerate() {
        let mut info: ParameterInfo = unsafe { std::mem::zeroed() };
        let rc = unsafe { host.controller.getParameterInfo(i as i32, &mut info) };
        assert_eq!(rc, kResultOk, "{}", p.key);
        assert_eq!(info.id, p.id, "{}", p.key);
        let title = unsafe { crate::strings::read_wstring(info.title.as_ptr()) };
        assert_eq!(title.as_deref(), Some(p.title));
        let read_only = info.flags & ParameterInfo_::ParameterFlags_::kIsReadOnly as i32 != 0;
        assert_eq!(read_only, p.read_only(), "{}", p.key);
    }

    // Every setting moved, saved, and read back by a fresh instance.
    let moved: Vec<f64> = spec
        .params
        .iter()
        .enumerate()
        .map(|(i, p)| p.normalized(p.plain(0.15 + 0.7 * (i as f64 * 0.37).fract())))
        .collect();
    for (p, &v) in spec.params.iter().zip(&moved) {
        host.set(p.id, v);
    }
    host.block(64);
    let saved = host.save();
    let fresh = Host::load::<P>(48_000.0, 256);
    assert!(fresh.restore(&saved), "a saved state loads");
    assert_eq!(fresh.save(), saved, "and saves the same again");
    for (p, &v) in spec.params.iter().zip(&moved) {
        let got = unsafe { fresh.controller.getParamNormalized(p.id) };
        assert!((got - v).abs() < 1e-9, "{}: {got} for {v}", p.key);
    }
    assert!(!fresh.restore(b"not a state"), "junk is refused");
}
