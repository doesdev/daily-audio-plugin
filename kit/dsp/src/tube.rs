//! A valve stage, with the things about a valve a person may want to
//! refine. Every plugin that models one uses this, so "the tube" means
//! the same four things everywhere and has the same window to set
//! them in. At rest (`TubeShape::default()`) it is a plain `tanh`.
//!
//! - **Bias**: where on the curve the valve rests. Off centre, the two
//!   halves of a wave are bent unequally, which is where a valve's
//!   even harmonics come from.
//! - **Grid**: how suddenly the top half gives out. A real grid starts
//!   to conduct and flattens positive swings harder than negative.
//! - **Sag**: the supply drooping under a sustained load, so a held
//!   loud sound eases back and blooms as it lets go.
//! - **Glow**: more of the top end goes into the curve and the same
//!   comes off after it, so the harmonics it adds sit high: the sheen.

use crate::pole;

/// How a valve is set, each from 0 to 1 but `bias`, from -1 to 1.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TubeShape {
    pub bias: f32,
    pub grid: f32,
    pub sag: f32,
    pub glow: f32,
}

/// The curve alone, with no memory: `x` through a valve whose top
/// half hardens with `grid`. At `grid` 0 it is `tanh`. It is odd in
/// nothing but that: the bottom half is always `tanh`.
#[inline]
pub fn tube_curve(x: f32, grid: f32) -> f32 {
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
    // The load the supply has seen lately.
    load: f32,
    load_pole: f32,
    // The low part of the signal, taken off to find its top end.
    low_in: f32,
    low_out: f32,
    glow_pole: f32,
}

/// How far the supply can droop: at full sag and full load, to a half.
const DROOP: f32 = 0.5;
/// How much of the top end glow adds going in: up to 12 dB.
const LIFT: f32 = 3.0;

impl Tube {
    pub fn new(rate: f32) -> Self {
        Self {
            shape: TubeShape::default(),
            load: 0.0,
            load_pole: pole(120.0, rate),
            low_in: 0.0,
            low_out: 0.0,
            // The top end starts about 2 kHz.
            glow_pole: (-std::f32::consts::TAU * 2000.0 / rate).exp(),
        }
    }

    pub fn reset(&mut self) {
        (self.load, self.low_in, self.low_out) = (0.0, 0.0, 0.0);
    }

    pub fn set(&mut self, shape: TubeShape) {
        self.shape = TubeShape {
            bias: shape.bias.clamp(-1.0, 1.0),
            grid: shape.grid.clamp(0.0, 1.0),
            sag: shape.sag.clamp(0.0, 1.0),
            glow: shape.glow.clamp(0.0, 1.0),
        };
    }

    /// One sample through the valve. `swing` is added to the bias for
    /// this sample: a plugin that moves the resting point itself, as
    /// Undertow does with one half's bass, passes that here. What the
    /// resting point alone would give is taken off, so silence in is
    /// silence out.
    #[inline]
    pub fn next(&mut self, x: f32, swing: f32) -> f32 {
        let TubeShape {
            bias,
            grid,
            sag,
            glow,
        } = self.shape;
        // Glow: lift the top going in.
        let lift = glow * LIFT;
        let follow = 1.0 - self.glow_pole;
        let lifted = x + (x - self.low_in) * lift;
        self.low_in += (x - self.low_in) * follow;
        // Sag: the more it has carried lately, the less it gives.
        let supply = 1.0 - sag * DROOP * self.load.min(1.0);
        let rest = bias + swing;
        let bent = tube_curve(lifted * supply + rest, grid) - tube_curve(rest, grid);
        self.load += (bent.abs() - self.load) * (1.0 - self.load_pole);
        // And take the same lift off again, exactly: the lift's own
        // sum turned round, following its own output as the lift
        // followed its input. A sound the curve leaves alone comes out
        // as it went in; what stays is what the curve added up there.
        let out = (bent + lift * self.low_out) / (1.0 + lift);
        self.low_out += (out - self.low_out) * follow;
        out
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
            assert_eq!(tube_curve(x, 0.0), x.tanh());
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
                (tube_curve(x, grid) - want).abs() < 1e-5,
                "{x} {grid}: {}",
                tube_curve(x, grid)
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
        let shape = TubeShape {
            bias: 0.7,
            grid: 1.0,
            sag: 1.0,
            glow: 1.0,
        };
        assert!(run(shape, 0.0, 100.0, 2000).iter().all(|&y| y == 0.0));
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
}
