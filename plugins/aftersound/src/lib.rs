//! Aftersound: every note is two strings, one in each channel, joined
//! at a shared bridge. Struck together they move as one, and the bridge
//! drinks that fast: the prompt sound, in the middle. Mistuned a hair,
//! they drift apart, and motion where they swing against each other
//! barely moves the bridge, so it rings on: the aftersound, in the
//! sides. The middle is the in-phase mode and the side the out-of-phase
//! mode, exactly, so a note starts narrow and blooms wide as it dies.

use kit::level::{db_to_gain, to_fraction};
use kit::{Asset, Block, Info, Param, Plugin, Role, meter, range};
use kit_dsp::{MidSide, Smooth};

// Indexes into PARAMS, for `Block`.
pub const PROMPT: usize = 0;
pub const AFTER: usize = 1;
pub const MISTUNE: usize = 2;
pub const STRIKE: usize = 3;
pub const BRIGHT: usize = 4;
pub const RELEASE: usize = 5;
pub const OUTPUT: usize = 6;
pub const BYPASS: usize = 7;
pub const BALANCE: usize = 8;
pub const LISTEN: usize = 9;
// Indexes into METERS.
pub const LEVEL: usize = 0;
pub const MID: usize = 1;
pub const SIDE: usize = 2;

// How fast a moved setting is followed, in ms.
const EASE_MS: f32 = 20.0;

/// How many notes ring at once. A key struck again re-strikes its own
/// strings, as a piano's hammer does.
pub const VOICES: usize = 16;

/// The pitch the picture and `foreseen` work at, A below middle C.
pub const REFERENCE_HZ: f32 = 220.0;

/// How far the bridge pulls on the strings' shared motion each time a
/// wave comes round, so that the in-phase mode dies in `prompt` seconds
/// (60 dB) while the out-of-phase mode, which the bridge does not feel,
/// dies in `after`. 0 is no pull; 0.5 takes it all in one pass.
pub fn coupling(prompt: f32, after: f32, hz: f32) -> f32 {
    let extra = (1.0 / prompt - 1.0 / after).max(0.0);
    (0.5 * (1.0 - 10f32.powf(-3.0 * extra / hz))).clamp(0.0, 0.5)
}

/// How much a wave keeps each time it comes round, to die by 60 dB in
/// `seconds` at `hz`.
pub fn keeps(seconds: f32, hz: f32) -> f32 {
    10f32.powf(-3.0 / (seconds.max(1e-3) * hz))
}

/// The middle's and the side's level `seconds` after a strike, for a
/// note at `REFERENCE_HZ`, worked out a period at a time with each
/// string as one phasor: the law the sound follows, at its fundamental,
/// and the curve the picture draws (`foreseen` in `ui/main.js` is the
/// same, held to it by a test on each side). `strike` is -1 to 1.
pub fn foreseen(prompt: f32, after: f32, mistune: f32, strike: f32, seconds: f32) -> (f32, f32) {
    let hz = REFERENCE_HZ;
    let (alpha, g) = (coupling(prompt, after, hz), keeps(after, hz));
    // Each string turns against the note by half the mistuning.
    let turn = std::f32::consts::PI * mistune / hz;
    let (c, s) = (turn.cos(), turn.sin());
    let (mut a, mut b) = ([1.0 + strike, 0.0], [1.0 - strike, 0.0]);
    for _ in 0..(seconds * hz).round() as usize {
        let ra = [g * (a[0] * c + a[1] * s), g * (a[1] * c - a[0] * s)];
        let rb = [g * (b[0] * c - b[1] * s), g * (b[1] * c + b[0] * s)];
        let pull = [alpha * (ra[0] + rb[0]), alpha * (ra[1] + rb[1])];
        a = [ra[0] - pull[0], ra[1] - pull[1]];
        b = [rb[0] - pull[0], rb[1] - pull[1]];
    }
    let half = |x: f32, y: f32| 0.5 * x.hypot(y);
    (
        half(a[0] + b[0], a[1] + b[1]),
        half(a[0] - b[0], a[1] - b[1]),
    )
}

// The lowest a string is tuned: below this a key plays this.
const LOWEST_HZ: f32 = 20.0;

// The most a wave may keep in a pass, so no loop ever gains.
const MOST_KEPT: f32 = 0.999_99;

// Below this a voice is silent and free to play another note.
const QUIET: f32 = 1e-6;

// How loud a full-velocity strike is, before the output trim.
const STRIKE_LEVEL: f32 = 0.3;

/// How long a partial at 2 kHz rings, in seconds, at `bright` (0 to
/// 1): from a felt-dark thud to a wire that sings.
pub fn partial_seconds(bright: f32) -> f32 {
    0.12 * 50f32.powf(bright)
}

/// The string's low-pass for a note at `hz`, set so a partial at 2 kHz
/// (or the second partial, if that is higher) dies in
/// `partial_seconds`, never later than the note itself. A low string
/// goes round many more times a second than a high one, so the same
/// filter would dull it far sooner: this is worked out per note.
pub fn dulling(bright: f32, after: f32, hz: f32, rate: f32) -> f32 {
    let seconds = partial_seconds(bright).min(after);
    let up = (2000.0f32).max(2.0 * hz).min(0.45 * rate);
    // What the partial may keep each pass beyond what the note keeps.
    let p = 10f32.powf(-3.0 * (1.0 / seconds - 1.0 / after).max(0.0) / hz);
    let c = (std::f32::consts::TAU * up / rate).cos();
    let p2 = p * p;
    if 1.0 - p2 < 1e-9 {
        return 0.0;
    }
    // |H| at `up` equal to p, for H(z) = (1 - a) / (1 - a z^-1).
    let b = 1.0 - p2 * c;
    ((b - (b * b - (1.0 - p2) * (1.0 - p2)).max(0.0).sqrt()) / (1.0 - p2)).clamp(0.0, 0.95)
}

/// One string: a delay as long as its period, a first-order allpass
/// for the part of a sample that a delay cannot give, and a one-pole
/// low-pass for the upper partials dying sooner than the fundamental.
struct Wire {
    line: Vec<f32>,
    at: usize,
    taps: usize,
    eta: f32,
    ap_x: f32,
    ap_y: f32,
    dull: f32,
    lp_y: f32,
    // What the low-pass takes from the fundamental each pass, so that
    // what the settings ask of the fundamental is what it does.
    lp_gain: f32,
}

impl Wire {
    fn new(length: usize) -> Self {
        Self {
            line: vec![0.0; length.max(4)],
            at: 0,
            taps: 1,
            eta: 0.0,
            ap_x: 0.0,
            ap_y: 0.0,
            dull: 0.0,
            lp_y: 0.0,
            lp_gain: 1.0,
        }
    }

    /// Tunes the loop to `hz`: the delay, the allpass and the
    /// low-pass's own delay at that pitch add up to one period.
    fn tune(&mut self, hz: f32, rate: f32, dull: f32) {
        let w = std::f32::consts::TAU * hz / rate;
        let lp_delay = (dull * w.sin()).atan2(1.0 - dull * w.cos()) / w;
        let want = rate / hz - lp_delay;
        let most = self.line.len() - 1;
        let taps = ((want - 0.5).floor() as usize).clamp(1, most);
        let frac = (want - taps as f32).clamp(0.0, 1.99);
        self.taps = taps;
        self.eta = (1.0 - frac) / (1.0 + frac);
        self.dull = dull;
        self.lp_gain = (1.0 - dull) / (1.0 - 2.0 * dull * w.cos() + dull * dull).sqrt();
    }

    fn clear(&mut self) {
        self.line.fill(0.0);
        (self.ap_x, self.ap_y, self.lp_y) = (0.0, 0.0, 0.0);
    }

    /// The wave coming back to the bridge, a period after it left.
    #[inline]
    fn back(&mut self) -> f32 {
        let len = self.line.len();
        let x = self.line[(self.at + len - self.taps) % len];
        let y = self.eta * x + self.ap_x - self.eta * self.ap_y;
        (self.ap_x, self.ap_y) = (x, y);
        self.lp_y = (1.0 - self.dull) * y + self.dull * self.lp_y;
        self.lp_y
    }

    /// The wave leaving the bridge.
    #[inline]
    fn send(&mut self, x: f32) {
        self.line[self.at] = x;
        self.at = (self.at + 1) % self.line.len();
    }
}

/// One note: two strings at a shared bridge, and the hammer on them.
struct Voice {
    key: u8,
    hz: f32,
    sounding: bool,
    released: bool,
    pedalled: bool,
    wires: [Wire; 2],
    // The bridge's pull on the strings' shared motion, and what each
    // string keeps per pass (held, then damped).
    pull: f32,
    kept: [f32; 2],
    damped: [f32; 2],
    // The hammer: where it is in its stroke, how long that is, how
    // hard, and how it divides between the strings.
    stroke: usize,
    stroke_len: usize,
    force: f32,
    share: [f32; 2],
    level: f32,
}

impl Voice {
    fn new(length: usize) -> Self {
        Self {
            key: 0,
            hz: REFERENCE_HZ,
            sounding: false,
            released: false,
            pedalled: false,
            wires: [Wire::new(length), Wire::new(length)],
            pull: 0.0,
            kept: [0.0; 2],
            damped: [0.0; 2],
            stroke: 0,
            stroke_len: 1,
            force: 0.0,
            share: [1.0; 2],
            level: 0.0,
        }
    }

    /// What each pass keeps and how hard the bridge pulls, from the
    /// settings as they stand.
    fn settle(&mut self, s: &Settings) {
        self.pull = coupling(s.prompt, s.after, self.hz);
        let held = keeps(s.after, self.hz);
        let damp = keeps(s.release, self.hz);
        for (k, wire) in self.wires.iter().enumerate() {
            self.kept[k] = (held / wire.lp_gain).min(MOST_KEPT);
            self.damped[k] = (held.min(damp) / wire.lp_gain).min(MOST_KEPT);
        }
    }

    fn strike(&mut self, key: u8, velocity: u8, s: &Settings, rate: f32) {
        let v = velocity as f32 / 127.0;
        if !self.sounding || self.key != key {
            self.wires.iter_mut().for_each(Wire::clear);
            self.key = key;
            self.hz = (440.0 * 2f32.powf((key as f32 - 69.0) / 12.0)).max(LOWEST_HZ);
            let dull = dulling(s.bright, s.after, self.hz, rate);
            let half = 0.5 * s.mistune;
            self.wires[0].tune(self.hz - half, rate, dull);
            self.wires[1].tune(self.hz + half, rate, dull);
            self.level = 0.0;
        }
        self.sounding = true;
        self.released = false;
        self.pedalled = false;
        self.settle(s);
        // A harder hammer is a shorter blow, never more than half a
        // period, so the blow does not cancel itself round the loop.
        let ms = (0.4 + 3.6 * (1.0 - s.bright)) * (1.3 - 0.6 * v);
        let most = 0.5 * rate / self.hz;
        self.stroke_len = ((ms * 0.001 * rate).min(most) as usize).max(2);
        self.stroke = 0;
        self.force = STRIKE_LEVEL * v.powf(1.5);
        // Toward 100%, the right-hand string, as a pan would go.
        self.share = [1.0 - s.strike, 1.0 + s.strike];
    }

    /// One sample of both strings.
    #[inline]
    fn next(&mut self) -> (f32, f32) {
        let kept = match self.released {
            true => self.damped,
            false => self.kept,
        };
        let a = kept[0] * self.wires[0].back();
        let b = kept[1] * self.wires[1].back();
        let pull = self.pull * (a + b);
        let blow = match self.stroke < self.stroke_len {
            true => {
                let u = std::f32::consts::TAU * self.stroke as f32 / self.stroke_len as f32;
                self.stroke += 1;
                // One cycle, so the blow leaves nothing standing still
                // on the strings.
                self.force * u.sin() * 0.5 * (1.0 - u.cos())
            }
            false => 0.0,
        };
        let (l, r) = (
            a - pull + blow * self.share[0],
            b - pull + blow * self.share[1],
        );
        self.wires[0].send(l);
        self.wires[1].send(r);
        self.level = l.abs().max(r.abs()).max(self.level * 0.9995);
        if self.level < QUIET && self.stroke >= self.stroke_len {
            self.sounding = false;
        }
        (l, r)
    }
}

/// The settings a voice reads, in their working terms.
struct Settings {
    prompt: f32,
    after: f32,
    mistune: f32,
    strike: f32,
    bright: f32,
    release: f32,
}

// Takes the still-standing part out, so nothing slow gathers.
#[derive(Clone, Copy)]
struct DcBlock {
    x: f32,
    y: f32,
    r: f32,
}

impl DcBlock {
    fn new(rate: f32) -> Self {
        let r = 1.0 - std::f32::consts::TAU * 8.0 / rate;
        Self { x: 0.0, y: 0.0, r }
    }

    #[inline]
    fn next(&mut self, x: f32) -> f32 {
        self.y = x - self.x + self.r * self.y;
        self.x = x;
        self.y
    }
}

pub struct Aftersound {
    rate: f32,
    voices: Vec<Voice>,
    sustain: bool,
    dc: [DcBlock; 2],
    halves: MidSide,
    output: Smooth,
}

impl Aftersound {
    fn settings(block: &Block) -> Settings {
        Settings {
            prompt: block.value(PROMPT),
            after: block.value(AFTER),
            mistune: block.value(MISTUNE),
            strike: block.value(STRIKE) / 100.0,
            bright: block.value(BRIGHT) / 100.0,
            release: block.value(RELEASE),
        }
    }

    fn hear(&mut self, data: [u8; 3], s: &Settings) {
        let (kind, key, value) = (data[0] & 0xF0, data[1], data[2]);
        match (kind, value) {
            (0x90, v) if v > 0 => self.press(key, v, s),
            (0x80, _) | (0x90, _) => self.lift(key),
            // The sustain pedal, where the host hands it over.
            (0xB0, _) if key == 64 => {
                self.sustain = value >= 64;
                if !self.sustain {
                    for voice in self.voices.iter_mut().filter(|v| v.pedalled) {
                        voice.released = true;
                        voice.pedalled = false;
                    }
                }
            }
            // All sound off, all notes off.
            (0xB0, _) if key == 120 || key == 123 => {
                self.sustain = false;
                for voice in &mut self.voices {
                    voice.released = true;
                }
            }
            _ => {}
        }
    }

    fn press(&mut self, key: u8, velocity: u8, s: &Settings) {
        let same = self.voices.iter().position(|v| v.sounding && v.key == key);
        let free = || self.voices.iter().position(|v| !v.sounding);
        let quietest = || {
            (0..self.voices.len())
                .min_by(|&a, &b| self.voices[a].level.total_cmp(&self.voices[b].level))
                .unwrap_or(0)
        };
        let slot = same.or_else(free).unwrap_or_else(quietest);
        let rate = self.rate;
        if let Some(voice) = self.voices.get_mut(slot) {
            voice.strike(key, velocity, s, rate);
        }
    }

    fn lift(&mut self, key: u8) {
        let sustain = self.sustain;
        for voice in self
            .voices
            .iter_mut()
            .filter(|v| v.sounding && v.key == key)
        {
            match sustain {
                true => voice.pedalled = true,
                false => voice.released = true,
            }
        }
    }
}

impl Plugin for Aftersound {
    const INFO: &'static Info = &Info {
        name: "Aftersound",
        vendor: kit::VENDOR,
        url: "https://github.com/doesdev/daily-audio-plugin",
        description: "Two coupled strings a note, one in each channel",
        version: env!("CARGO_PKG_VERSION"),
        // Fixed for good: hosts save projects by these three.
        clap_id: "dev.doesdev.aftersound",
        vst3_processor: [0xE95F_CF58, 0x6DBC_2DE4, 0x9DB5_9D06, 0x2A39_A6E4],
        vst3_controller: [0x6BCB_5FCC, 0x0E8D_B15A, 0x8F7A_2C71, 0x20A3_2F52],
        role: Role::Instrument,
        audio_in: 0,
        audio_out: 2,
        midi_in: true,
        midi_out: false,
        size: (980, 540),
        min_size: (735, 405),
    };

    // The ids are fixed for good too. A new setting takes the next id
    // and goes wherever it reads best; none is ever reused.
    const PARAMS: &'static [Param] = &[
        range(0, "prompt", "Prompt", "Strings", (0.05, 4.0, 0.6), "s")
            .skew(2.0)
            .decimals(2)
            .about("How fast the middle dies: the bridge drinking what the strings do together"),
        range(1, "after", "Aftersound", "Strings", (0.5, 30.0, 7.0), "s")
            .skew(2.0)
            .decimals(2)
            .about("How long the sides ring: what the strings do against each other"),
        range(2, "mistune", "Mistune", "Strings", (0.0, 4.0, 0.5), "Hz")
            .decimals(2)
            .about(
                "How far apart the two strings are tuned: the more, the sooner the sides take over",
            ),
        range(3, "strike", "Strike", "Hammer", (-100.0, 100.0, 0.0), "%")
            .decimals(0)
            .about("Which string the hammer hits harder: off centre, the sides start at once"),
        range(4, "bright", "Brightness", "Hammer", (0.0, 100.0, 60.0), "%")
            .decimals(0)
            .about("How hard the hammer is and how long the upper partials last"),
        range(5, "release", "Release", "Strings", (0.05, 3.0, 0.25), "s")
            .skew(2.0)
            .decimals(2)
            .about("How fast the damper stops a note once its key is let go"),
        range(6, "output", "Output", "Output", (-24.0, 12.0, 0.0), "dB")
            .decimals(1)
            .about("Level after everything else"),
        // Ids 7 to 9 are the meters'.
        kit::bypass(10),
        kit::balance(11),
        kit::listen(12),
    ];

    const METERS: &'static [Param] = &[
        meter(7, "level", "Out"),
        meter(8, "mid", "Mid"),
        meter(9, "side", "Side"),
    ];

    const ASSETS: &'static [Asset] = &[
        ("/main.js", "text/javascript", include_str!("../ui/main.js")),
        ("/style.css", "text/css", include_str!("../ui/style.css")),
        (
            "/foreseen.js",
            "text/javascript",
            include_str!("../ui/foreseen.js"),
        ),
    ];

    fn new() -> Self {
        let mut me = Self {
            rate: 48_000.0,
            voices: Vec::new(),
            sustain: false,
            dc: [DcBlock::new(48_000.0); 2],
            halves: MidSide::new(48_000.0),
            output: Smooth::new(1.0, EASE_MS, 48_000.0),
        };
        me.prepare(48_000.0, 512);
        me
    }

    fn prepare(&mut self, rate: f32, _max_block: usize) {
        let at = |index: usize| Self::PARAMS[index].default as f32;
        self.rate = rate;
        // Long enough for the lowest string, mistuned down.
        let length = (rate / (LOWEST_HZ - 2.5)) as usize + 4;
        self.voices = (0..VOICES).map(|_| Voice::new(length)).collect();
        self.sustain = false;
        self.dc = [DcBlock::new(rate); 2];
        self.halves = MidSide::new(rate);
        self.output = Smooth::new(db_to_gain(at(OUTPUT)), EASE_MS, rate);
    }

    fn reset(&mut self) {
        for voice in &mut self.voices {
            voice.wires.iter_mut().for_each(Wire::clear);
            voice.sounding = false;
        }
        self.sustain = false;
        self.dc = [DcBlock::new(self.rate); 2];
    }

    fn tail(&self) -> u32 {
        // The longest aftersound the settings allow.
        (30.0 * self.rate) as u32
    }

    fn process(&mut self, block: &mut Block) {
        let s = Self::settings(block);
        let output = db_to_gain(block.value(OUTPUT));
        let balance = block.value(BALANCE) / 100.0;
        let listen = block.choice(LISTEN);
        for voice in self.voices.iter_mut().filter(|v| v.sounding) {
            voice.settle(&s);
        }

        let (mut loudest, mut mids, mut sides) = (0.0f32, 0.0f32, 0.0f32);
        let mut next_event = 0;
        for i in 0..block.frames() {
            while next_event < block.events.len() && block.events[next_event].offset as usize <= i {
                let data = block.events[next_event].data;
                self.hear(data, &s);
                next_event += 1;
            }
            let (mut l, mut r) = (0.0f32, 0.0f32);
            for voice in self.voices.iter_mut().filter(|v| v.sounding) {
                let (a, b) = voice.next();
                l += a;
                r += b;
            }
            let trim = self.output.next(output);
            let (l, r) = (self.dc[0].next(l) * trim, self.dc[1].next(r) * trim);
            // Last of all, which half is heard and how they weigh.
            let (gm, gs) = self.halves.next(balance, listen);
            let (mid, side) = (0.5 * (l + r) * gm, 0.5 * (l - r) * gs);
            let (l, r) = (mid + side, mid - side);
            mids = mids.max(mid.abs());
            sides = sides.max(side.abs());
            loudest = loudest.max(l.abs()).max(r.abs());
            block.output(0)[i] = l;
            if block.outputs() > 1 {
                block.output(1)[i] = r;
            }
        }
        // Anything stamped past the block's end lands on its last sample.
        while next_event < block.events.len() {
            let data = block.events[next_event].data;
            self.hear(data, &s);
            next_event += 1;
        }
        block.meter(LEVEL, to_fraction(loudest));
        block.meter(MID, to_fraction(mids));
        block.meter(SIDE, to_fraction(sides));
    }
}

kit_vst3::export!(Aftersound);
kit_clap::export!(Aftersound);
