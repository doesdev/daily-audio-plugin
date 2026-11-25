//! Runs a plugin a block at a time, whatever the host: the format
//! crates and the tests all go through `Runner`. It owns the buffers,
//! so a plugin's input and output never alias, and it cuts a block
//! longer than the host promised into pieces that fit.

use crate::{Param, Plugin};

/// MIDI messages a block may carry each way; more are dropped.
pub const EVENTS: usize = 1024;

/// A MIDI 1.0 message at a sample of its block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub offset: u32,
    pub data: [u8; 3],
}

/// Where the host's transport stood at the start of a block.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Transport {
    pub playing: bool,
    pub tempo: Option<f64>,
    /// Quarter notes from the project's start.
    pub beats: Option<f64>,
    /// Samples from the project's start.
    pub sample: Option<i64>,
}

/// One block, as `Plugin::process` sees it.
pub struct Block<'a> {
    frames: usize,
    ins: &'a [Vec<f32>],
    outs: &'a mut [Vec<f32>],
    params: &'a [f64],
    spec: &'static [Param],
    out: &'a mut Vec<Event>,
    meters: &'a mut [f64],
    /// Notes and controllers in, in order of their offsets.
    pub events: &'a [Event],
    pub transport: Transport,
    pub rate: f32,
}

impl Block<'_> {
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// Input channels this block has; an instrument's has none.
    pub fn inputs(&self) -> usize {
        self.ins.len()
    }

    pub fn outputs(&self) -> usize {
        self.outs.len()
    }

    /// Input channel `ch`; a mono input answers for every channel.
    pub fn input(&self, ch: usize) -> &[f32] {
        match self.ins.get(ch).or(self.ins.last()) {
            Some(buf) => &buf[..self.frames],
            None => &[],
        }
    }

    /// Output channel `ch`, silent until written.
    pub fn output(&mut self, ch: usize) -> &mut [f32] {
        &mut self.outs[ch][..self.frames]
    }

    /// Channel `ch` in and out together.
    pub fn io(&mut self, ch: usize) -> (&[f32], &mut [f32]) {
        let input = match self.ins.get(ch).or(self.ins.last()) {
            Some(buf) => &buf[..self.frames],
            None => &[],
        };
        (input, &mut self.outs[ch][..self.frames])
    }

    /// Setting `index` as the host holds it, 0 to 1.
    pub fn norm(&self, index: usize) -> f64 {
        self.params.get(index).copied().unwrap_or(0.0)
    }

    /// Setting `index` in its own terms.
    pub fn value(&self, index: usize) -> f32 {
        self.spec
            .get(index)
            .map_or(0.0, |p| p.plain(self.norm(index)) as f32)
    }

    /// Which choice a list is on.
    pub fn choice(&self, index: usize) -> usize {
        self.spec
            .get(index)
            .map_or(0, |p| p.index(self.norm(index)))
    }

    pub fn on(&self, index: usize) -> bool {
        self.choice(index) > 0
    }

    /// Sends a MIDI message out at `offset`. Past `EVENTS` in a block
    /// it is dropped rather than allocate.
    pub fn send(&mut self, offset: usize, data: [u8; 3]) {
        if self.out.len() < self.out.capacity() {
            let offset = offset.min(self.frames.saturating_sub(1)) as u32;
            self.out.push(Event { offset, data });
        }
    }

    /// Reports meter `index` for this block, 0 to 1.
    pub fn meter(&mut self, index: usize, value: f64) {
        if let Some(slot) = self.meters.get_mut(index) {
            *slot = value.clamp(0.0, 1.0);
        }
    }
}

pub struct Runner<P: Plugin> {
    plugin: P,
    rate: f32,
    max: usize,
    ins: Vec<Vec<f32>>,
    outs: Vec<Vec<f32>>,
    /// Every setting, normalized; the host side writes, `run` reads.
    pub params: Vec<f64>,
    pending: Vec<Event>,
    piece: Vec<Event>,
    sent: Vec<Event>,
    meters: Vec<f64>,
}

impl<P: Plugin> Runner<P> {
    /// A prepared plugin. Allocates, so never on the audio thread.
    pub fn new(rate: f32, max_block: usize) -> Self {
        let max = max_block.max(1);
        let mut plugin = P::new();
        plugin.prepare(rate, max);
        let buffers = |n: u32| (0..n).map(|_| vec![0.0; max]).collect();
        Self {
            plugin,
            rate,
            max,
            ins: buffers(P::INFO.audio_in),
            outs: buffers(P::INFO.audio_out),
            params: P::PARAMS.iter().map(Param::default_norm).collect(),
            pending: Vec::with_capacity(EVENTS),
            piece: Vec::with_capacity(EVENTS),
            sent: Vec::with_capacity(EVENTS),
            meters: vec![0.0; P::METERS.len()],
        }
    }

    pub fn rate(&self) -> f32 {
        self.rate
    }

    pub fn max_block(&self) -> usize {
        self.max
    }

    pub fn latency(&self) -> u32 {
        self.plugin.latency()
    }

    pub fn tail(&self) -> u32 {
        self.plugin.tail()
    }

    pub fn reset(&mut self) {
        self.plugin.reset();
    }

    /// What the plugin last reported, by meter index.
    pub fn meters(&self) -> &[f64] {
        &self.meters
    }

    /// A MIDI message for the next `run`, at an offset in its block.
    pub fn push(&mut self, event: Event) {
        if self.pending.len() < self.pending.capacity() {
            self.pending.push(event);
        }
    }

    /// Runs `frames` of audio. `read(ch, start, into)` fills a piece of
    /// input channel `ch` from the host's block; `write(ch, start,
    /// from)` takes a piece of output; `send` takes each MIDI message
    /// out, its offset counted from the block's start. No allocation.
    pub fn run(
        &mut self,
        frames: usize,
        transport: Transport,
        mut read: impl FnMut(usize, usize, &mut [f32]),
        mut write: impl FnMut(usize, usize, &[f32]),
        mut send: impl FnMut(Event),
    ) {
        self.pending.sort_unstable_by_key(|e| e.offset);
        let mut start = 0;
        let mut transport = transport;
        while start < frames {
            let n = (frames - start).min(self.max);
            let last = start + n >= frames;
            for (ch, buf) in self.ins.iter_mut().enumerate() {
                read(ch, start, &mut buf[..n]);
            }
            self.outs.iter_mut().for_each(|buf| buf[..n].fill(0.0));

            self.piece.clear();
            self.sent.clear();
            let here = |e: &&Event| {
                let at = e.offset as usize;
                at >= start && (at < start + n || last)
            };
            for e in self.pending.iter().filter(here) {
                let offset = (e.offset as usize - start).min(n - 1) as u32;
                self.piece.push(Event { offset, ..*e });
            }

            self.plugin.process(&mut Block {
                frames: n,
                ins: &self.ins,
                outs: &mut self.outs,
                params: &self.params,
                spec: P::PARAMS,
                out: &mut self.sent,
                meters: &mut self.meters,
                events: &self.piece,
                transport,
                rate: self.rate,
            });

            for (ch, buf) in self.outs.iter().enumerate() {
                write(ch, start, &buf[..n]);
            }
            for e in &self.sent {
                send(Event {
                    offset: e.offset + start as u32,
                    ..*e
                });
            }
            transport.sample = transport.sample.map(|s| s + n as i64);
            start += n;
        }
        self.pending.clear();
    }

    /// Runs interleaved-by-channel buffers, for tests and tools:
    /// `input[ch]` in, the same shape out. Allocates the result.
    pub fn render(&mut self, input: &[Vec<f32>], frames: usize, block: usize) -> Vec<Vec<f32>> {
        let mut out = vec![vec![0.0; frames]; P::INFO.audio_out as usize];
        let mut start = 0;
        while start < frames {
            let n = (frames - start).min(block.max(1));
            let at = start;
            self.run(
                n,
                Transport::default(),
                |ch, s, into| match input.get(ch).or(input.last()) {
                    Some(buf) => into.copy_from_slice(&buf[at + s..at + s + into.len()]),
                    None => into.fill(0.0),
                },
                |ch, s, from| out[ch][at + s..at + s + from.len()].copy_from_slice(from),
                |_| {},
            );
            start += n;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Info, Role, meter, range};

    // Doubles what it hears, echoes each note a sample late, and
    // reports the block's length as a meter.
    struct Double;

    impl Plugin for Double {
        const INFO: &'static Info = &Info {
            name: "Double",
            vendor: "kit",
            url: "",
            description: "",
            version: "0.0.0",
            clap_id: "dev.kit.double",
            vst3_processor: [1, 2, 3, 4],
            vst3_controller: [5, 6, 7, 8],
            role: Role::Effect,
            audio_in: 2,
            audio_out: 2,
            midi_in: true,
            midi_out: true,
            size: (400, 300),
            min_size: (200, 100),
        };
        const PARAMS: &'static [Param] = &[range(0, "gain", "Gain", "", (0.0, 4.0, 2.0), "")];
        const METERS: &'static [Param] = &[meter(1, "len", "Length").fraction()];

        fn new() -> Self {
            Self
        }

        fn prepare(&mut self, _rate: f32, _max_block: usize) {}

        fn process(&mut self, block: &mut Block) {
            let gain = block.value(0);
            for ch in 0..block.outputs() {
                let (input, output) = block.io(ch);
                output
                    .iter_mut()
                    .zip(input)
                    .for_each(|(o, i)| *o = i * gain);
            }
            for i in 0..block.events.len() {
                let e = block.events[i];
                block.send(e.offset as usize + 1, e.data);
            }
            let frames = block.frames();
            block.meter(0, frames as f64 / 100.0);
        }
    }

    #[test]
    fn a_long_block_goes_through_in_pieces_with_its_notes_in_place() {
        let mut runner = Runner::<Double>::new(48_000.0, 16);
        let input: Vec<f32> = (0..40).map(|i| i as f32).collect();
        let mut out = vec![vec![0.0; 40]; 2];
        let mut sent = Vec::new();
        for offset in [35, 3, 16] {
            runner.push(Event {
                offset,
                data: [0x90, 60, 100],
            });
        }

        runner.run(
            40,
            Transport::default(),
            |_, start, into| into.copy_from_slice(&input[start..start + into.len()]),
            |ch, start, from| out[ch][start..start + from.len()].copy_from_slice(from),
            |e| sent.push(e.offset),
        );

        let want: Vec<f32> = input.iter().map(|s| s * 2.0).collect();
        assert_eq!(out, vec![want.clone(), want]);
        assert_eq!(sent, vec![4, 17, 36], "each a sample late, in order");
        assert_eq!(runner.meters(), [0.08], "the last piece was 8 long");
    }

    #[test]
    fn follows_its_settings_and_forgets_sent_notes() {
        let mut runner = Runner::<Double>::new(48_000.0, 64);
        runner.params[0] = 0.25;
        let out = runner.render(&[vec![1.0; 10]], 10, 4);

        assert_eq!(out[1], vec![1.0; 10], "a mono input feeds both, at gain 1");
        let mut sent = 0;
        runner.run(
            4,
            Transport::default(),
            |_, _, _| {},
            |_, _, _| {},
            |_| sent += 1,
        );
        assert_eq!(sent, 0);
    }
}
