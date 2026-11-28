//! A valve stage, with the things about a valve a person may want to
//! refine. Every plugin that models one uses this, so "the tube" means
//! the same things everywhere and has the same window to set them in.
//! At rest (`TubeShape::default()`) it is a plain `tanh`.
//!
//! The levers are the causes that change what a valve stage sounds
//! like, as `docs/tone.md` sets out, not a list of part numbers:
//!
//! - **Stage**: how the valve is wired. A pentode's screen holds its
//!   current steady, so its curve is even either side and what it adds
//!   is mostly odd harmonics. A triode follows a three-halves power
//!   law, lopsided from the first, so it adds the second harmonic at
//!   any level, cuts off on one side, and its top is a little darker
//!   (the Miller effect). A pentode strapped as a triode sits between.
//! - **Bias**: where on the curve the valve rests, added to whatever
//!   the plugin itself does with the resting point.
//! - **Grid**: how suddenly the top half gives out. A real grid starts
//!   to conduct and flattens positive swings harder than negative.
//! - **Bloom**: a grid that conducts charges the capacitor feeding it,
//!   which pushes the resting point down for a moment after a loud
//!   sound and lets it come back: the bloom after a hit.
//! - **Feedback**: a cathode left unbypassed feeds the stage back on
//!   itself. Cleaner and firmer up to the ceiling, and then a harder
//!   knee into it.
//! - **Sag**: the supply drooping under a sustained load.
//! - **Glow**: more of the top end goes into the curve and the same
//!   comes off after it, so the harmonics it adds sit high: the sheen.

use crate::pole;

/// How the valve is wired, as `TubeShape::stage` and the Stage setting
/// count them.
pub const TRIODE: usize = 0;
pub const STRAPPED: usize = 1;
pub const PENTODE: usize = 2;

/// How a valve is set: `stage` is one of the three above, `bias` runs
/// from -1 to 1, and the rest from 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TubeShape {
    pub stage: usize,
    pub bias: f32,
    pub grid: f32,
    pub bloom: f32,
    pub feedback: f32,
    pub sag: f32,
    pub glow: f32,
}

impl Default for TubeShape {
    fn default() -> Self {
        Self {
            stage: PENTODE,
            bias: 0.0,
            grid: 0.0,
            bloom: 0.0,
            feedback: 0.0,
            sag: 0.0,
            glow: 0.0,
        }
    }
}

// How far below its resting point a stage cuts off, in the curve's own
// units: the nearer, the more lopsided. A pentode has no such point.
fn cutoff(stage: usize) -> Option<f32> {
    match stage {
        TRIODE => Some(2.5),
        STRAPPED => Some(6.0),
        _ => None,
    }
}

// Where the top of a stage starts to fall away, in hertz: the Miller
// effect, which a pentode's screen spares it.
fn miller(stage: usize) -> Option<f32> {
    match stage {
        TRIODE => Some(14_000.0),
        STRAPPED => Some(20_000.0),
        _ => None,
    }
}

/// The curve alone, with no memory: `x` through a valve wired as
/// `stage`, whose top half hardens with `grid`. A pentode at `grid` 0
/// is `tanh`. A triode's current follows the three-halves power of how
/// far it is above cutoff, scaled to rise as steeply at rest as the
/// pentode does, and the same ceiling then bounds it.
#[inline]
pub fn tube_curve(x: f32, grid: f32, stage: usize) -> f32 {
    let x = match cutoff(stage) {
        Some(far) => {
            let up = (1.0 + x / far).max(0.0);
            far / 1.5 * (up * up.sqrt() - 1.0)
        }
        None => x,
    };
    let soft = x.tanh();
    if x <= 0.0 || grid <= 0.0 {
        return soft;
    }
    // The grid conducting: the same rise, and a ceiling that comes
    // down from 1 to two thirds as it hardens.
    let ceiling = 1.0 - grid / 3.0;
    let hard = ceiling * (x / ceiling).tanh();
    soft + (hard - soft) * grid
}

#[derive(Debug, Clone, Copy)]
pub struct Tube {
    shape: TubeShape,
    rate: f32,
    // The load the supply has seen lately.
    load: f32,
    load_pole: f32,
    // The low part of the signal, taken off to find its top end.
    low_in: f32,
    low_out: f32,
    glow_pole: f32,
    // How far grid current has pushed the resting point down, and how
    // fast that charges and lets go.
    shift: f32,
    charge: f32,
    release: f32,
    // The last sample out of the curve: where the feedback's answer
    // is looked for first.
    last: f32,
    // The Miller effect's one pole, if this stage has one.
    top: f32,
    top_pole: Option<f32>,
    // How many times the feedback's answer is looked for again.
    turns: u8,
}

/// How far the supply can droop: at full sag and full load, to a half.
const DROOP: f32 = 0.5;
/// How much of the top end glow adds going in: up to 12 dB.
const LIFT: f32 = 3.0;
/// How far up its curve a grid starts to draw current.
const CONDUCTS: f32 = 0.5;
/// How many times the feedback's answer is looked for again when
/// playing, and when rendering, where time is not short.
pub const PLAYING_TURNS: u8 = 3;
pub const RENDERING_TURNS: u8 = 8;
/// How much of itself a fully unbypassed cathode feeds back: 12 dB.
const LOOP: f32 = 3.0;

impl Tube {
    pub fn new(rate: f32) -> Self {
        let mut made = Self {
            shape: TubeShape::default(),
            rate,
            load: 0.0,
            load_pole: pole(120.0, rate),
            low_in: 0.0,
            low_out: 0.0,
            // The top end starts about 2 kHz.
            glow_pole: (-std::f32::consts::TAU * 2000.0 / rate).exp(),
            shift: 0.0,
            charge: 1.0 - pole(2.0, rate),
            release: 0.0,
            last: 0.0,
            top: 0.0,
            top_pole: None,
            turns: PLAYING_TURNS,
        };
        made.set(TubeShape::default());
        made
    }

    /// How closely the feedback's answer is found: `PLAYING_TURNS`
    /// unless told otherwise.
    pub fn turns(&mut self, turns: u8) {
        self.turns = turns.max(1);
    }

    /// How far up the curve a sample `x` reaches with the resting
    /// point moved by `swing`, as `Need` wants it: the lift, the
    /// feedback's make-up and the bias all carry it further, and a
    /// hardened grid brings the ceiling nearer. A triode has a corner
    /// where it cuts off, and near that nothing is smooth, which
    /// this says with infinity.
    #[inline]
    pub fn reach(&self, x: f32, swing: f32) -> f32 {
        let TubeShape {
            stage,
            bias,
            grid,
            feedback,
            glow,
            ..
        } = self.shape;
        let far = x.abs() * (1.0 + glow * LIFT) * (1.0 + feedback * LOOP)
            + (bias + swing).abs()
            + self.shift;
        let far = far / (1.0 - grid / 3.0);
        // A grid that has started to harden joins two curves, and
        // the join is not as smooth as either.
        let far = if grid > 0.0 { far * 2.0 } else { far };
        match cutoff(stage) {
            Some(off) if far > 0.6 * off => f32::INFINITY,
            Some(off) => far / (1.0 - far / off),
            None => far,
        }
    }

    pub fn reset(&mut self) {
        (self.load, self.low_in, self.low_out) = (0.0, 0.0, 0.0);
        (self.shift, self.last, self.top) = (0.0, 0.0, 0.0);
    }

    pub fn set(&mut self, shape: TubeShape) {
        self.shape = TubeShape {
            stage: shape.stage.min(PENTODE),
            bias: shape.bias.clamp(-1.0, 1.0),
            grid: shape.grid.clamp(0.0, 1.0),
            bloom: shape.bloom.clamp(0.0, 1.0),
            feedback: shape.feedback.clamp(0.0, 1.0),
            sag: shape.sag.clamp(0.0, 1.0),
            glow: shape.glow.clamp(0.0, 1.0),
        };
        // The more it blooms the longer it takes to come back: 30 ms
        // to 300 ms, as a coupling capacitor and its grid leak would.
        self.release = 1.0 - pole(30.0 + 270.0 * self.shape.bloom, self.rate);
        // Only where the rate leaves room for it.
        self.top_pole = miller(self.shape.stage)
            .filter(|hz| *hz < self.rate * 0.45)
            .map(|hz| (-std::f32::consts::TAU * hz / self.rate).exp());
    }

    // `a` through the curve from where it rests, less what the rest
    // alone gives. With feedback, part of what comes out is taken from
    // what goes in, and what goes in is made up so that a small sound
    // keeps its level: the answer is where the two agree, found by
    // Newton's method from the sample before.
    #[inline]
    fn bend(&mut self, a: f32, rest: f32) -> f32 {
        let TubeShape {
            stage,
            grid,
            feedback,
            ..
        } = self.shape;
        let floor = tube_curve(rest, grid, stage);
        if feedback <= 0.0 {
            return tube_curve(a + rest, grid, stage) - floor;
        }
        let back = feedback * LOOP;
        let given = a * (1.0 + back) + rest;
        let mut y = self.last;
        for _ in 0..self.turns {
            let at = given - back * y;
            let here = tube_curve(at, grid, stage);
            let slope = (tube_curve(at + 1e-3, grid, stage) - here) * 1e3;
            y -= (here - floor - y) / (-back * slope - 1.0);
        }
        self.last = y.clamp(-2.0, 2.0);
        self.last
    }

    /// One sample through the valve. `swing` is added to the bias for
    /// this sample: a plugin that moves the resting point itself, as
    /// Undertow does with one half's bass, passes that here, and the
    /// Bias setting only ever adds to it. What the resting point alone
    /// would give is taken off, so silence in is silence out.
    #[inline]
    pub fn next(&mut self, x: f32, swing: f32) -> f32 {
        let TubeShape {
            bias,
            bloom,
            sag,
            glow,
            ..
        } = self.shape;
        // Glow: lift the top going in.
        let lift = glow * LIFT;
        let follow = 1.0 - self.glow_pole;
        let lifted = x + (x - self.low_in) * lift;
        self.low_in += (x - self.low_in) * follow;
        // Sag: the more it has carried lately, the less it gives.
        let supply = 1.0 - sag * DROOP * self.load.min(1.0);
        let driven = lifted * supply;
        // Bloom: what the grid drew has pushed the rest down.
        let rest = bias + swing - self.shift;
        let bent = self.bend(driven, rest);
        if bloom > 0.0 {
            let drawn = (driven + bias + swing - CONDUCTS).max(0.0) * bloom;
            let rate = match drawn > self.shift {
                true => self.charge,
                false => self.release,
            };
            self.shift += (drawn.min(1.5) - self.shift) * rate;
        } else {
            self.shift = 0.0;
        }
        self.load += (bent.abs() - self.load) * (1.0 - self.load_pole);
        // And take the same lift off again, exactly: the lift's own
        // sum turned round, following its own output as the lift
        // followed its input. A sound the curve leaves alone comes out
        // as it went in; what stays is what the curve added up there.
        let out = (bent + lift * self.low_out) / (1.0 + lift);
        self.low_out += (out - self.low_out) * follow;
        match self.top_pole {
            Some(pole) => {
                self.top += (out - self.top) * (1.0 - pole);
                self.top
            }
            None => out,
        }
    }
}

impl crate::Bend for Tube {
    fn retune(&mut self, rate: f32) {
        self.rate = rate;
        self.load_pole = pole(120.0, rate);
        self.glow_pole = (-std::f32::consts::TAU * 2000.0 / rate).exp();
        self.charge = 1.0 - pole(2.0, rate);
        self.set(self.shape);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    fn run(shape: TubeShape, level: f32, hz: f32, n: usize) -> Vec<f32> {
        let mut tube = Tube::new(RATE);
        tube.set(shape);
        (0..n)
            .map(|i| {
                let t = i as f32 / RATE;
                tube.next(level * (std::f32::consts::TAU * hz * t).sin(), 0.0)
            })
            .collect()
    }

    // How much of `x` is at `hz`, by correlation.
    fn at(x: &[f32], hz: f32) -> f32 {
        let (mut re, mut im) = (0.0, 0.0);
        for (i, v) in x.iter().enumerate() {
            let phase = std::f32::consts::TAU * hz * i as f32 / RATE;
            re += v * phase.cos();
            im += v * phase.sin();
        }
        (re * re + im * im).sqrt() * 2.0 / x.len() as f32
    }

    #[test]
    fn at_rest_it_is_tanh_and_nothing_else() {
        let mut tube = Tube::new(RATE);
        for x in [-3.0, -0.4, 0.0, 0.2, 1.5] {
            assert_eq!(tube.next(x, 0.0), x.tanh(), "{x}");
            assert_eq!(tube_curve(x, 0.0, PENTODE), x.tanh());
        }
    }

    #[test]
    fn the_curve_is_as_the_page_draws_it() {
        // The same numbers are in kit/ui/gallery.test.js: the window's
        // picture and the sound must be one curve.
        let cases = [
            (0.5, 0.0, 0.462_117),
            (0.5, 1.0, 0.423_433),
            (2.0, 0.5, 0.891_878),
            (-2.0, 1.0, -0.964_028),
            (3.0, 1.0, 0.666_502),
        ];
        for (x, grid, want) in cases {
            assert!(
                (tube_curve(x, grid, PENTODE) - want).abs() < 1e-5,
                "{x} {grid}: {}",
                tube_curve(x, grid, PENTODE)
            );
        }
    }

    #[test]
    fn bias_and_grid_add_the_even_harmonics_a_centred_curve_lacks() {
        let second = |shape| at(&run(shape, 0.5, 100.0, 48_000)[4800..], 200.0);
        let plain = second(TubeShape::default());
        let biased = second(TubeShape {
            bias: 0.4,
            ..TubeShape::default()
        });
        let gridded = second(TubeShape {
            grid: 1.0,
            ..TubeShape::default()
        });

        assert!(plain < 1e-4, "{plain}");
        assert!(biased > 0.01, "{biased}");
        assert!(gridded > 0.005, "{gridded}");
    }

    #[test]
    fn silence_in_is_silence_out_however_it_is_set() {
        for stage in [TRIODE, STRAPPED, PENTODE] {
            let shape = TubeShape {
                stage,
                bias: 0.7,
                grid: 1.0,
                bloom: 1.0,
                feedback: 1.0,
                sag: 1.0,
                glow: 1.0,
            };
            let out = run(shape, 0.0, 100.0, 2000);
            assert!(out.iter().all(|y| y.abs() < 1e-6), "{stage}");
        }
    }

    #[test]
    fn sag_eases_a_held_loud_sound_back_and_leaves_a_quiet_one() {
        let sagging = TubeShape {
            sag: 1.0,
            ..TubeShape::default()
        };
        let peak = |x: &[f32]| x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        let loud = run(sagging, 1.0, 100.0, 48_000);
        let stiff = run(TubeShape::default(), 1.0, 100.0, 48_000);
        let quiet = run(sagging, 0.01, 100.0, 48_000);

        // The first cycle is as loud as without; a second later it
        // has eased back.
        assert!((peak(&loud[..480]) - peak(&stiff[..480])).abs() < 0.05);
        assert!(peak(&loud[43_200..]) < peak(&stiff[43_200..]) * 0.9);
        assert!((peak(&quiet[43_200..]) - 0.01).abs() < 0.001);
    }

    #[test]
    fn glow_puts_more_of_what_the_curve_adds_up_high() {
        let glowing = TubeShape {
            glow: 1.0,
            ..TubeShape::default()
        };
        // The third harmonic of a 3 kHz tone, against the tone itself.
        let third = |shape| {
            let y = run(shape, 0.5, 3000.0, 48_000);
            at(&y[4800..], 9000.0) / at(&y[4800..], 3000.0)
        };

        assert!(third(glowing) > third(TubeShape::default()) * 2.0);
        // And a quiet tone, which the curve leaves alone, comes out
        // as it went in: the lift is taken off again.
        let quiet = run(glowing, 0.001, 3000.0, 48_000);
        assert!((at(&quiet[4800..], 3000.0) - 0.001).abs() < 0.000_05);
    }

    #[test]
    fn the_stages_curves_are_as_the_page_draws_them() {
        // The same numbers are in kit/ui/gallery.test.js.
        let cases = [
            (0.5, 0.0, TRIODE, 0.480_953),
            (-1.0, 0.0, TRIODE, -0.712_415),
            (-3.0, 0.0, TRIODE, -0.931_110),
            (1.0, 1.0, TRIODE, 0.618_432),
            (0.5, 0.0, STRAPPED, 0.470_160),
            (-2.0, 0.5, STRAPPED, -0.949_105),
        ];
        for (x, grid, stage, want) in cases {
            let got = tube_curve(x, grid, stage);
            assert!((got - want).abs() < 1e-5, "{x} {grid} {stage}: {got}");
        }
    }

    #[test]
    fn a_triode_adds_the_second_harmonic_at_rest_and_a_pentode_the_third() {
        let wired = |stage| {
            let y = run(
                TubeShape {
                    stage,
                    ..TubeShape::default()
                },
                0.3,
                100.0,
                48_000,
            );
            (at(&y[4800..], 200.0), at(&y[4800..], 300.0))
        };
        let (triode2, _) = wired(TRIODE);
        let (strapped2, _) = wired(STRAPPED);
        let (pentode2, pentode3) = wired(PENTODE);

        assert!(pentode2 < 1e-4 && pentode3 > 0.001, "{pentode2} {pentode3}");
        assert!(triode2 > 0.004, "{triode2}");
        // Strapped sits between the two.
        assert!(strapped2 > pentode2 + 0.001 && strapped2 < triode2 * 0.7);
    }

    #[test]
    fn a_triodes_top_is_a_little_darker_and_a_pentodes_is_not() {
        let kept = |stage| {
            let y = run(
                TubeShape {
                    stage,
                    ..TubeShape::default()
                },
                0.01,
                12_000.0,
                48_000,
            );
            at(&y[4800..], 12_000.0) / 0.01
        };
        assert!((kept(PENTODE) - 1.0).abs() < 0.01);
        assert!(kept(TRIODE) < 0.9 && kept(TRIODE) > 0.6, "{}", kept(TRIODE));
    }

    #[test]
    fn feedback_keeps_a_small_sound_and_cleans_a_larger_one() {
        let fed = TubeShape {
            feedback: 1.0,
            ..TubeShape::default()
        };
        let third = |shape, level| {
            let y = run(shape, level, 100.0, 48_000);
            (at(&y[4800..], 100.0), at(&y[4800..], 300.0))
        };
        let (small, _) = third(fed, 0.01);
        assert!((small - 0.01).abs() < 0.000_1, "{small}");
        let (_, open) = third(TubeShape::default(), 0.5);
        let (_, closed) = third(fed, 0.5);
        assert!(closed < open * 0.4, "{closed} against {open}");
    }

    #[test]
    fn bloom_pushes_the_rest_down_after_a_loud_sound_and_lets_it_back() {
        let blooming = TubeShape {
            bloom: 1.0,
            ..TubeShape::default()
        };
        let mut tube = Tube::new(RATE);
        tube.set(blooming);
        let tone =
            |i: usize, level: f32| level * (std::f32::consts::TAU * 1000.0 * i as f32 / RATE).sin();
        // A tenth of a second driven hard, then a quiet tone. Off its
        // centre the curve gives the quiet tone a second harmonic,
        // which is how the push shows.
        (0..4800).for_each(|i| {
            tube.next(tone(i, 2.0), 0.0);
        });
        let after: Vec<f32> = (4800..96_000)
            .map(|i| tube.next(tone(i, 0.1), 0.0))
            .collect();
        let second = |from: usize| at(&after[from..from + 2400], 2000.0);

        assert!(second(0) > 0.000_5, "{}", second(0));
        assert!(second(86_000) < second(0) * 0.05, "{}", second(86_000));

        // Without bloom nothing is pushed.
        let mut plain = Tube::new(RATE);
        (0..4800).for_each(|i| {
            plain.next(tone(i, 2.0), 0.0);
        });
        let dry: Vec<f32> = (4800..9600)
            .map(|i| plain.next(tone(i, 0.1), 0.0))
            .collect();
        assert!(at(&dry[..2400], 2000.0) < 1e-4);
    }
}
