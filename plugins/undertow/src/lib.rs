//! Undertow: the sides go through a valve-like curve whose resting
//! point is set, sample by sample, by the low end of the middle. The
//! middle passes untouched; what the bass put on the curve is taken
//! off again after it, so a mono sound comes out as it went in.

use kit::level::{db_to_gain, peak, to_fraction};
use kit::{Asset, Block, Info, Param, Plugin, Role, list, meter, range};
use kit_dsp::{Smooth, Svf};

// Indexes into PARAMS, for `Block`.
pub const PUSH: usize = 0;
pub const FOLLOW: usize = 1;
pub const DRIVE: usize = 2;
pub const STEER: usize = 3;
pub const MIX: usize = 4;
pub const OUTPUT: usize = 5;
pub const BYPASS: usize = 6;
// Indexes into METERS.
pub const INPUT: usize = 0;
pub const LEVEL: usize = 1;
pub const LEAN: usize = 2;
pub const SWING: usize = 3;

// Steer: which half leans on the other.
pub const MID_ON_SIDE: usize = 0;
pub const SIDE_ON_MID: usize = 1;

// How fast a moved setting is followed, in ms.
const EASE_MS: f32 = 20.0;

// Push at 100% puts a full-scale low end this far along the curve:
// far enough to reach the flat of it.
const REACH: f32 = 3.0;

// The lean meter holds its peak this long, so the picture's point
// is readable at the rate an editor redraws.
const LEAN_HOLD_MS: f32 = 60.0;

/// The valve's curve: smooth, odd and level at its centre, so at rest
/// it only rounds the sides and adds no even harmonics of its own.
pub fn curve(x: f32) -> f32 {
    x.tanh()
}

/// One sample of the carried half `x`, driven by `drive` (a gain) with
/// the curve's resting point moved to `bias`. What the bias alone
/// would give is taken off, so silence in gives silence out whatever
/// the bias, and the result is scaled back so a small sound at rest
/// comes out at its own level.
pub fn leaned(x: f32, drive: f32, bias: f32) -> f32 {
    (curve(x * drive + bias) - curve(bias)) / drive
}

pub struct Undertow {
    rate: f32,
    // The low end of the half that leans, which becomes the bias.
    low: Svf,
    push: Smooth,
    drive: Smooth,
    mix: Smooth,
    output: Smooth,
    lean: f32,
    lean_fall: f32,
}

impl Plugin for Undertow {
    const INFO: &'static Info = &Info {
        name: "Undertow",
        vendor: "doesdev",
        url: "https://github.com/doesdev/daily-audio-plugin",
        description: "The middle's low end biases the sides' valve",
        version: env!("CARGO_PKG_VERSION"),
        // Fixed for good: hosts save projects by these three.
        clap_id: "dev.doesdev.undertow",
        vst3_processor: [0x8D90_BCE9, 0x59A0_01C4, 0xA086_8586, 0x4C51_D271],
        vst3_controller: [0xDB88_ADD7, 0xC030_1314, 0xB1EE_9227, 0x4459_001B],
        role: Role::Effect,
        audio_in: 2,
        audio_out: 2,
        midi_in: false,
        midi_out: false,
        size: (820, 500),
        min_size: (615, 375),
    };

    // The ids are fixed for good too. A new setting takes the next id
    // and goes wherever it reads best; none is ever reused.
    const PARAMS: &'static [Param] = &[
        range(0, "push", "Push", "Lean", (0.0, 100.0, 50.0), "%")
            .decimals(0)
            .about("How far the low end of one half moves the other's resting point on the curve"),
        range(1, "follow", "Follow", "Lean", (30.0, 1000.0, 160.0), "Hz")
            .skew(2.5)
            .step(1.0)
            .decimals(0)
            .about("How much of the leaning half does the pushing: everything below this"),
        range(2, "drive", "Drive", "Valve", (0.0, 24.0, 6.0), "dB")
            .decimals(1)
            .about("How hard the carried half goes into the curve"),
        list(
            3,
            "steer",
            "Steer",
            "Lean",
            &["Mid on side", "Side on mid"],
            MID_ON_SIDE,
        )
        .about("Which half leans: the middle's bass on the sides, or the sides on the middle"),
        range(4, "mix", "Mix", "Output", (0.0, 100.0, 100.0), "%")
            .decimals(0)
            .about("How much of the leaned half against its own dry signal"),
        range(5, "output", "Output", "Output", (-24.0, 12.0, 0.0), "dB")
            .decimals(1)
            .about("Level after everything else"),
        // Ids 6 to 9 are the meters'.
        kit::bypass(10),
    ];

    const METERS: &'static [Param] = &[
        meter(6, "input", "In"),
        meter(7, "level", "Out"),
        meter(8, "lean", "Lean").fraction(),
        meter(9, "swing", "Swing").fraction(),
    ];

    const ASSETS: &'static [Asset] = &[
        ("/main.js", "text/javascript", include_str!("../ui/main.js")),
        ("/style.css", "text/css", include_str!("../ui/style.css")),
    ];

    fn new() -> Self {
        let rate = 48_000.0;
        Self {
            rate,
            low: Svf::new(rate),
            push: Smooth::new(0.0, EASE_MS, rate),
            drive: Smooth::new(1.0, EASE_MS, rate),
            mix: Smooth::new(0.0, EASE_MS, rate),
            output: Smooth::new(1.0, EASE_MS, rate),
            lean: 0.0,
            lean_fall: 0.0,
        }
    }

    fn prepare(&mut self, rate: f32, _max_block: usize) {
        let at = |index: usize| Self::PARAMS[index].default as f32;
        self.rate = rate;
        self.low = Svf::new(rate);
        self.push = Smooth::new(at(PUSH) / 100.0, EASE_MS, rate);
        self.drive = Smooth::new(db_to_gain(at(DRIVE)), EASE_MS, rate);
        self.mix = Smooth::new(at(MIX) / 100.0, EASE_MS, rate);
        self.output = Smooth::new(db_to_gain(at(OUTPUT)), EASE_MS, rate);
        self.lean = 0.0;
        self.lean_fall = 1.0 / (LEAN_HOLD_MS * 0.001 * rate);
    }

    fn reset(&mut self) {
        self.low.reset();
        self.lean = 0.0;
    }

    fn process(&mut self, block: &mut Block) {
        let push = block.value(PUSH) / 100.0 * REACH;
        let follow = block.value(FOLLOW);
        let drive = db_to_gain(block.value(DRIVE));
        let mix = block.value(MIX) / 100.0;
        let output = db_to_gain(block.value(OUTPUT));
        let mid_leans = block.choice(STEER) == MID_ON_SIDE;
        // The filter's corner moves once a block: it shapes the bias,
        // never the sound directly, so a step in it is not heard.
        self.low.set(follow, 0.707);

        let mut heard = 0.0f32;
        for ch in 0..block.inputs() {
            heard = heard.max(peak(block.input(ch)));
        }
        let mut swing = 0.0f32;
        for i in 0..block.frames() {
            let (push, drive) = (self.push.next(push), self.drive.next(drive));
            let (mix, output) = (self.mix.next(mix), self.output.next(output));
            let (left, right) = (sample(block, 0, i), sample(block, 1, i));
            let (mid, side) = ((left + right) * 0.5, (left - right) * 0.5);
            let (key, carried) = if mid_leans { (mid, side) } else { (side, mid) };

            // The bias is the leaning half's own low wave, not an
            // envelope of it: up and down push opposite ways.
            let bias = self.low.next(key).low * push;
            let wet = leaned(carried, drive, bias);
            let carried = carried + (wet - carried) * mix;

            self.lean = (self.lean - self.lean_fall).max(bias.abs() / REACH);
            swing = swing.max((carried * drive).abs() / REACH);
            let (mid, side) = if mid_leans {
                (key, carried)
            } else {
                (carried, key)
            };
            let outs = [(mid + side) * output, (mid - side) * output];
            for (ch, value) in outs.iter().enumerate().take(block.outputs()) {
                block.output(ch)[i] = *value;
            }
        }
        let mut loudest = 0.0f32;
        for ch in 0..block.outputs() {
            loudest = loudest.max(peak(block.output(ch)));
        }
        block.meter(INPUT, to_fraction(heard));
        block.meter(LEVEL, to_fraction(loudest));
        block.meter(LEAN, self.lean.min(1.0) as f64);
        block.meter(SWING, swing.min(1.0) as f64);
    }
}

// Sample `i` of input `ch`; a mono input answers for both sides.
fn sample(block: &Block, ch: usize, i: usize) -> f32 {
    let ch = ch.min(block.inputs().saturating_sub(1));
    block.input(ch).get(i).copied().unwrap_or(0.0)
}

kit_vst3::export!(Undertow);
kit_clap::export!(Undertow);
