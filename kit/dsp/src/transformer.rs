//! An audio transformer, with the things about one a person may want
//! to refine. Every plugin that models one uses this, so "the iron"
//! means the same four things everywhere and has the same window to
//! set them in. At rest (`IronShape::default()`) it does nothing.
//!
//! A transformer's core saturates on flux, which is the voltage
//! added up over time: so the lows, which stay one way for longer,
//! saturate first and the highs pass clean. That is the weight.
//!
//! - **Weight**: how hard the low end is driven into the core.
//! - **Core**: below what frequency the core is working.
//! - **Grip**: how much the core stays magnetised one way. A core
//!   that carries a standing current, as one in a single-ended stage
//!   does, bends the two halves of a low wave unequally, which adds
//!   the even harmonics a balanced core lacks.
//! - **Roll**: the top end the windings give away.

/// How a transformer is set. `core` is in hertz; the rest 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IronShape {
    pub weight: f32,
    pub core: f32,
    pub grip: f32,
    pub roll: f32,
}

impl Default for IronShape {
    fn default() -> Self {
        Self {
            weight: 0.0,
            core: 120.0,
            grip: 0.0,
            roll: 0.0,
        }
    }
}

/// The core's curve alone, with no memory: what the low end `x` comes
/// out as at `weight`. At `weight` 0 it is `x`.
#[inline]
pub fn iron_curve(x: f32, weight: f32) -> f32 {
    if weight <= 0.0 {
        return x;
    }
    // Driven up to 12 dB into the core, and scaled back so a small
    // sound keeps its level.
    let drive = 1.0 + weight * 3.0;
    (x * drive).tanh() / drive
}

#[derive(Debug, Clone, Copy)]
pub struct Transformer {
    shape: IronShape,
    rate: f32,
    low: f32,
    low_pole: f32,
    top: f32,
    top_pole: f32,
}

/// How far a fully gripped core stands off centre.
const STANDING: f32 = 0.35;

fn one_pole(hz: f32, rate: f32) -> f32 {
    (-std::f32::consts::TAU * hz / rate).exp()
}

impl Transformer {
    pub fn new(rate: f32) -> Self {
        let mut made = Self {
            shape: IronShape::default(),
            rate,
            low: 0.0,
            low_pole: 0.0,
            top: 0.0,
            top_pole: 0.0,
        };
        made.set(IronShape::default());
        made
    }

    pub fn reset(&mut self) {
        (self.low, self.top) = (0.0, 0.0);
    }

    pub fn set(&mut self, shape: IronShape) {
        self.shape = IronShape {
            weight: shape.weight.clamp(0.0, 1.0),
            core: shape.core.clamp(20.0, 1000.0),
            grip: shape.grip.clamp(0.0, 1.0),
            roll: shape.roll.clamp(0.0, 1.0),
        };
        self.low_pole = one_pole(self.shape.core, self.rate);
        // From out of hearing down to 5 kHz.
        let corner = 5000.0 * (self.rate * 0.45 / 5000.0).powf(1.0 - self.shape.roll);
        self.top_pole = one_pole(corner, self.rate);
    }

    #[inline]
    pub fn next(&mut self, x: f32) -> f32 {
        let IronShape {
            weight, grip, roll, ..
        } = self.shape;
        if weight <= 0.0 && grip <= 0.0 && roll <= 0.0 {
            return x;
        }
        // The lows go through the core; the rest goes round it.
        self.low += (x - self.low) * (1.0 - self.low_pole);
        let rest = x - self.low;
        // Grip: the core already leans one way before the sound comes.
        // What the lean alone gives is taken off: silence stays silent.
        let lean = grip * STANDING;
        let cored = iron_curve(self.low + lean, weight) - iron_curve(lean, weight);
        let out = cored + rest;
        if roll <= 0.0 {
            return out;
        }
        self.top += (out - self.top) * (1.0 - self.top_pole);
        self.top
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

    fn at(x: &[f32], hz: f32) -> f32 {
        let (mut re, mut im) = (0.0, 0.0);
        for (i, v) in x.iter().enumerate() {
            let phase = std::f32::consts::TAU * hz * i as f32 / RATE;
            re += v * phase.cos();
            im += v * phase.sin();
        }
        (re * re + im * im).sqrt() * 2.0 / x.len() as f32
    }

    fn heavy() -> IronShape {
        IronShape {
            weight: 1.0,
            ..IronShape::default()
        }
    }

    #[test]
    fn at_rest_it_passes_the_sound_exactly() {
        let mut iron = Transformer::new(RATE);
        for x in [-2.0, -0.3, 0.0, 0.7, 5.0] {
            assert_eq!(iron.next(x), x);
            assert_eq!(iron_curve(x, 0.0), x);
        }
    }

    #[test]
    fn the_curve_is_as_the_page_draws_it() {
        // The same numbers are in kit/ui/gallery.test.js.
        let cases = [
            (0.5, 1.0, 0.241_007),
            (0.1, 0.5, 0.097_967),
            (-1.0, 1.0, -0.249_832),
        ];
        for (x, weight, want) in cases {
            assert!(
                (iron_curve(x, weight) - want).abs() < 1e-5,
                "{}",
                iron_curve(x, weight)
            );
        }
    }

    #[test]
    fn the_lows_saturate_and_the_highs_pass_clean() {
        let third = |hz: f32| {
            let y = run(heavy(), 0.8, hz);
            at(&y[4800..], hz * 3.0) / at(&y[4800..], hz)
        };

        assert!(third(50.0) > 0.03, "{}", third(50.0));
        assert!(third(4000.0) < 0.001, "{}", third(4000.0));
    }

    #[test]
    fn the_core_sets_how_far_up_the_weight_reaches() {
        let third = |core: f32| {
            let y = run(IronShape { core, ..heavy() }, 0.8, 200.0);
            at(&y[4800..], 600.0) / at(&y[4800..], 200.0)
        };

        assert!(third(600.0) > third(60.0) * 3.0);
    }

    #[test]
    fn grip_makes_even_harmonics_and_roll_takes_the_top() {
        let gripping = IronShape {
            grip: 1.0,
            ..heavy()
        };
        let second = |shape| {
            let y = run(shape, 0.8, 50.0);
            at(&y[24_000..], 100.0)
        };
        assert!(second(gripping) > second(heavy()) + 1e-4);

        let rolled = IronShape {
            roll: 1.0,
            ..IronShape::default()
        };
        let top = at(&run(rolled, 0.5, 12_000.0)[4800..], 12_000.0);
        let low = at(&run(rolled, 0.5, 200.0)[4800..], 200.0);
        assert!(top < 0.25, "{top}");
        assert!((low - 0.5).abs() < 0.01, "{low}");
    }

    #[test]
    fn silence_in_is_silence_out_however_it_is_set() {
        let shape = IronShape {
            weight: 1.0,
            core: 300.0,
            grip: 1.0,
            roll: 1.0,
        };
        assert!(run(shape, 0.0, 100.0).iter().all(|&y| y == 0.0));
    }
}
