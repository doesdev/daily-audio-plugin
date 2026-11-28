//! An audio transformer, with the things about one a person may want
//! to refine. Every plugin that models one uses this, so "the iron"
//! means the same things everywhere and has the same window to set
//! them in. At rest (`IronShape::default()`) it does nothing.
//!
//! The one fact everything here follows from: a core saturates on
//! flux, and flux is the voltage added up over time. So a low note,
//! which stays one way for longer, fills the core sooner than a high
//! one at the same level, and what a transformer adds rises as the
//! frequency falls. The sound is added up into flux, the core bends
//! the flux, and the bend is turned back into sound. `docs/tone.md`
//! says where each lever comes from.
//!
//! - **Drive**: how hard the sound is pushed into the core.
//! - **Size**: how much iron there is: the frequency at which a full
//!   level sound just fills the core. A small one fills in the low
//!   mids; a big one only in the deepest bass.
//! - **Iron**: what the core is made of. Nickel is clean until a
//!   sudden knee. Steel bends sooner and more gently, and lags behind
//!   the sound (hysteresis), which is heard even on quiet sounds.
//! - **Source**: what feeds it. The more the source resists, the more
//!   of the core's bending reaches the output and the less bass gets
//!   through, until the sound is loud enough to open the core up.
//! - **Grip**: a standing current in a gapped core, as a single-ended
//!   stage leaves in its output transformer: the core leans one way,
//!   which adds even harmonics, and loses some of its bass.
//! - **Top** and **Ring**: the windings resonate above the band. Top
//!   brings that down into hearing, and Ring is how little the load
//!   damps it: a smooth roll, or a peak with an edge.

use crate::{Svf, pole};

/// How a transformer is set. `size` is in hertz; the rest 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IronShape {
    pub drive: f32,
    pub size: f32,
    pub iron: f32,
    pub source: f32,
    pub grip: f32,
    pub top: f32,
    pub ring: f32,
}

impl Default for IronShape {
    fn default() -> Self {
        Self {
            drive: 0.0,
            size: 60.0,
            iron: 0.0,
            source: 0.0,
            grip: 0.0,
            top: 0.0,
            ring: 0.2,
        }
    }
}

/// How hard a full Drive pushes: 12 dB into the core.
pub const PUSH: f32 = 4.0;

/// The core's curve alone, with no memory: flux `x` driven by `push`
/// (Drive times `PUSH`) through a core of `iron`, 0 for nickel to 1
/// for steel. With no push it is `x`. Nickel holds straight and turns
/// suddenly; steel is `tanh`.
#[inline]
pub fn iron_curve(x: f32, push: f32, iron: f32) -> f32 {
    if push < 1e-4 {
        return x;
    }
    let u = x * push;
    let nickel = u / (1.0 + u * u * u * u).sqrt().sqrt();
    (nickel + (u.tanh() - nickel) * iron) / push
}

#[derive(Debug, Clone, Copy)]
pub struct Transformer {
    shape: IronShape,
    rate: f32,
    // The sound added up: the flux in the core, and how much of it
    // there has been lately.
    flux: f32,
    leak: f32,
    scale: f32,
    filled: f32,
    filled_pole: f32,
    // Where the core's own magnetisation has got to, lagging the flux.
    lag: f32,
    // The bend at the last sample, to turn the bend back into sound.
    bent: f32,
    // What the source's resistance holds back of the lows.
    held: f32,
    corner: f32,
    // The windings' resonance.
    top: Svf,
    top_mix: f32,
}

/// How far a fully gripped core stands off centre.
const STANDING: f32 = 0.35;
/// How far steel's magnetisation can lag the flux, and how much of
/// the lag is felt.
const LAG: f32 = 0.02;
const FELT: f32 = 0.5;

impl Transformer {
    pub fn new(rate: f32) -> Self {
        let mut made = Self {
            shape: IronShape::default(),
            rate,
            flux: 0.0,
            // The flux leaks away below about 1.5 Hz, so a standing
            // offset in the sound cannot fill the core for good.
            leak: (-std::f32::consts::TAU * 1.5 / rate).exp(),
            scale: 0.0,
            filled: 0.0,
            filled_pole: pole(30.0, rate),
            lag: 0.0,
            bent: 0.0,
            held: 0.0,
            corner: 0.0,
            top: Svf::new(rate),
            top_mix: 0.0,
        };
        made.set(IronShape::default());
        made
    }

    pub fn reset(&mut self) {
        (self.flux, self.filled, self.lag) = (0.0, 0.0, 0.0);
        (self.bent, self.held) = (0.0, 0.0);
        self.top.reset();
    }

    pub fn set(&mut self, shape: IronShape) {
        self.shape = IronShape {
            drive: shape.drive.clamp(0.0, 1.0),
            size: shape.size.clamp(20.0, 1000.0),
            iron: shape.iron.clamp(0.0, 1.0),
            source: shape.source.clamp(0.0, 1.0),
            grip: shape.grip.clamp(0.0, 1.0),
            top: shape.top.clamp(0.0, 1.0),
            ring: shape.ring.clamp(0.0, 1.0),
        };
        let IronShape {
            size,
            source,
            grip,
            top,
            ring,
            ..
        } = self.shape;
        // A full level sound at `size` hertz is a flux of one.
        self.scale = std::f32::consts::TAU * size / self.rate;
        // The lows are lost where the core's reactance falls to the
        // source's resistance. A gap, which is what lets a core carry
        // a standing current, costs it inductance and raises that too.
        let lost = (source * 0.4 + grip * 0.3) * size;
        self.corner = (std::f32::consts::TAU * lost / self.rate).min(1.0);
        // The resonance comes down from out of hearing to 8 kHz, and
        // the load damps it from a smooth roll to a peak of 6 dB.
        let open = self.rate * 0.45;
        self.top
            .set(8000.0 * (open / 8000.0).powf(1.0 - top), 0.5 + 1.5 * ring);
        // It comes in over the first tenth, so there is no step at 0.
        self.top_mix = (top * 10.0).min(1.0);
    }

    #[inline]
    pub fn next(&mut self, x: f32) -> f32 {
        let IronShape {
            drive,
            iron,
            source,
            grip,
            top,
            ..
        } = self.shape;
        if drive <= 0.0 && iron <= 0.0 && source <= 0.0 && grip <= 0.0 && top <= 0.0 {
            return x;
        }
        // The source and the gap hold back the lows. A core's
        // inductance rises with level before it saturates, so a loud
        // sound opens it up and the lows come through: the bloom.
        let x = match self.corner > 0.0 {
            true => {
                let opened = self.corner / (1.0 + 2.0 * self.filled.min(1.0));
                self.held += (x - self.held) * opened;
                x - self.held
            }
            false => x,
        };
        // Sound into flux.
        self.flux = self.flux * self.leak + x * self.scale;
        let flux = self.flux;
        let push = drive * PUSH;
        self.filled += ((flux * push.max(1.0)).abs() - self.filled) * (1.0 - self.filled_pole);
        // Steel lags: its magnetisation follows the flux only once
        // the flux has moved far enough, either way.
        let width = iron * LAG;
        self.lag = self.lag.clamp(flux - width, flux + width);
        let felt = flux + (self.lag - flux) * iron * FELT;
        // Grip: the core already leans one way before the sound comes.
        // What the lean alone gives is taken off: silence stays silent.
        let lean = grip * STANDING / push.max(1.0);
        let bent = iron_curve(felt + lean, push, iron) - iron_curve(lean, push, iron) - flux;
        // Only the bend is turned back into sound and added, so what
        // the core leaves alone comes out exactly as it went in. The
        // more the source resists, the more of the bend is heard.
        let out = x + (bent - self.bent) / self.scale * (1.0 + 2.0 * source);
        self.bent = bent;
        if top <= 0.0 {
            return out;
        }
        out + (self.top.next(out).low - out) * self.top_mix
    }
}

impl crate::Bend for Transformer {
    fn retune(&mut self, rate: f32) {
        // The flux is counted in the same units at any rate, so what
        // it holds carries over as it is.
        self.rate = rate;
        self.leak = (-std::f32::consts::TAU * 1.5 / rate).exp();
        self.filled_pole = pole(30.0, rate);
        self.top.retune(rate);
        self.set(self.shape);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    fn run(shape: IronShape, level: f32, hz: f32) -> Vec<f32> {
        let mut iron = Transformer::new(RATE);
        iron.set(shape);
        (0..48_000)
            .map(|i| iron.next(level * (std::f32::consts::TAU * hz * i as f32 / RATE).sin()))
            .collect()
    }

    // How much of the last half second of `x` is at `hz`.
    fn at(x: &[f32], hz: f32) -> f32 {
        let x = &x[24_000..];
        let (mut re, mut im) = (0.0, 0.0);
        for (i, v) in x.iter().enumerate() {
            let phase = std::f32::consts::TAU * hz * i as f32 / RATE;
            re += v * phase.cos();
            im += v * phase.sin();
        }
        (re * re + im * im).sqrt() * 2.0 / x.len() as f32
    }

    fn driven() -> IronShape {
        IronShape {
            drive: 1.0,
            ..IronShape::default()
        }
    }

    #[test]
    fn at_rest_it_passes_the_sound_exactly() {
        let mut iron = Transformer::new(RATE);
        for x in [-0.9, -0.1, 0.0, 0.3, 1.0] {
            assert_eq!(iron.next(x), x);
        }
    }

    #[test]
    fn the_curve_is_as_the_page_draws_it() {
        // The same numbers are in kit/ui/gallery.test.js: the window's
        // picture and the sound must be one curve.
        let cases = [
            (0.5, 4.0, 0.0, 0.246_240),
            (0.5, 4.0, 1.0, 0.241_007),
            (0.2, 2.0, 0.5, 0.194_357),
            (-1.0, 4.0, 1.0, -0.249_832),
            (0.3, 0.0, 1.0, 0.3),
        ];
        for (x, push, iron, want) in cases {
            let got = iron_curve(x, push, iron);
            assert!((got - want).abs() < 1e-5, "{x} {push} {iron}: {got}");
        }
    }

    #[test]
    fn what_it_adds_rises_as_the_frequency_falls() {
        // The same level at each: an octave down is twice the flux.
        let third = |hz: f32| at(&run(driven(), 0.1, hz), hz * 3.0) / 0.1;

        assert!(third(30.0) > 0.02, "{}", third(30.0));
        assert!(third(30.0) > third(60.0) * 1.5);
        assert!(third(60.0) > third(240.0) * 4.0);
        assert!(third(2000.0) < 1e-4, "{}", third(2000.0));
        // And the highs come through at their own level.
        let kept = at(&run(driven(), 0.5, 2000.0), 2000.0);
        assert!((kept - 0.5).abs() < 0.001, "{kept}");
    }

    #[test]
    fn a_smaller_core_fills_higher_up() {
        let third = |size: f32| {
            let shape = IronShape { size, ..driven() };
            at(&run(shape, 0.5, 200.0), 600.0)
        };
        assert!(third(400.0) > third(60.0) * 5.0);
    }

    #[test]
    fn nickel_is_clean_until_its_knee_and_steel_is_heard_on_a_quiet_sound() {
        let third = |iron: f32, level: f32| {
            let shape = IronShape { iron, ..driven() };
            at(&run(shape, level, 60.0), 180.0) / level
        };
        // Quiet: steel's lag is there, nickel adds next to nothing.
        assert!(third(1.0, 0.03) > third(0.0, 0.03) * 5.0);
        assert!(third(0.0, 0.03) < 1e-4, "{}", third(0.0, 0.03));
        // Loud: both have filled.
        assert!(third(0.0, 1.0) > 0.05 && third(1.0, 1.0) > 0.05);
    }

    #[test]
    fn a_source_that_resists_shows_more_of_the_bend_and_less_of_the_bass() {
        let fed = |source: f32| IronShape { source, ..driven() };
        let third = |source: f32| at(&run(fed(source), 0.3, 60.0), 180.0);
        assert!(third(1.0) > third(0.0) * 1.5);

        // A quiet low note is held back; a loud one opens the core.
        let alone = |level: f32| {
            let shape = IronShape {
                source: 1.0,
                ..IronShape::default()
            };
            at(&run(shape, level, 20.0), 20.0) / level
        };
        assert!(alone(0.001) < 0.75, "{}", alone(0.001));
        assert!(alone(1.0) > alone(0.001) + 0.1, "{}", alone(1.0));
    }

    #[test]
    fn grip_adds_even_harmonics_and_takes_some_of_the_bass() {
        let gripped = IronShape {
            grip: 1.0,
            ..driven()
        };
        let second = |shape| at(&run(shape, 0.5, 60.0), 120.0);
        assert!(second(driven()) < 0.002, "{}", second(driven()));
        assert!(second(gripped) > 0.02, "{}", second(gripped));

        let quiet = IronShape {
            grip: 1.0,
            ..IronShape::default()
        };
        let kept = at(&run(quiet, 0.001, 20.0), 20.0) / 0.001;
        assert!(kept < 0.85, "{kept}");
    }

    #[test]
    fn the_top_rolls_smoothly_damped_and_peaks_when_it_rings() {
        let kept = |ring: f32, hz: f32| {
            let shape = IronShape {
                top: 1.0,
                ring,
                ..IronShape::default()
            };
            at(&run(shape, 0.1, hz), hz) / 0.1
        };
        // At the resonance itself, 8 kHz with Top full.
        assert!(kept(0.0, 8000.0) < 0.6, "{}", kept(0.0, 8000.0));
        assert!(kept(1.0, 8000.0) > 1.7, "{}", kept(1.0, 8000.0));
        // Well below it nothing has changed, and well above it has gone.
        assert!((kept(1.0, 500.0) - 1.0).abs() < 0.02);
        assert!(kept(1.0, 20_000.0) < 0.3);
    }

    #[test]
    fn silence_in_is_silence_out_however_it_is_set() {
        let shape = IronShape {
            drive: 1.0,
            size: 300.0,
            iron: 1.0,
            source: 1.0,
            grip: 1.0,
            top: 1.0,
            ring: 1.0,
        };
        assert!(run(shape, 0.0, 100.0).iter().all(|y| y.abs() < 1e-9));
    }
}
