//! A state variable filter (the trapezoidal form, after Zavalishin and
//! Simper): low, band and high outputs at once from one pass, stable
//! while its cutoff is moved every sample.

use crate::TINY;
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvfOut {
    pub low: f32,
    pub band: f32,
    pub high: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Svf {
    rate: f32,
    g: f32,
    k: f32,
    ic1: f32,
    ic2: f32,
}

impl Svf {
    pub fn new(rate: f32) -> Self {
        let mut svf = Self {
            rate,
            g: 0.0,
            k: 0.0,
            ic1: 0.0,
            ic2: 0.0,
        };
        svf.set(1000.0, 0.707);
        svf
    }

    /// Cutoff in Hz, held under Nyquist, and resonance as Q.
    pub fn set(&mut self, hz: f32, q: f32) {
        let hz = hz.clamp(10.0, self.rate * 0.49);
        self.g = (PI * hz / self.rate).tan();
        self.k = 1.0 / q.max(0.05);
    }

    pub fn reset(&mut self) {
        self.ic1 = 0.0;
        self.ic2 = 0.0;
    }

    #[inline]
    pub fn next(&mut self, x: f32) -> SvfOut {
        let a1 = 1.0 / (1.0 + self.g * (self.g + self.k));
        let a2 = self.g * a1;
        let a3 = self.g * a2;
        let v3 = x - self.ic2;
        let v1 = a1 * self.ic1 + a2 * v3;
        let v2 = self.ic2 + a2 * self.ic1 + a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1 + TINY;
        self.ic2 = 2.0 * v2 - self.ic2 + TINY;
        SvfOut {
            low: v2,
            band: v1,
            high: x - self.k * v1 - v2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The steady level of each output for a sine at `hz`.
    fn levels(cutoff: f32, hz: f32) -> SvfOut {
        let rate = 48_000.0;
        let mut svf = Svf::new(rate);
        svf.set(cutoff, 0.707);
        let mut peak = SvfOut {
            low: 0.0,
            band: 0.0,
            high: 0.0,
        };
        for i in 0..rate as usize {
            let out = svf.next((2.0 * PI * hz * i as f32 / rate).sin());
            if i > rate as usize / 2 {
                peak.low = peak.low.max(out.low.abs());
                peak.band = peak.band.max(out.band.abs());
                peak.high = peak.high.max(out.high.abs());
            }
        }
        peak
    }

    #[test]
    fn each_output_passes_its_own_side_of_the_cutoff() {
        let below = levels(1000.0, 100.0);
        let above = levels(1000.0, 10_000.0);
        let at = levels(1000.0, 1000.0);

        assert!(below.low > 0.98 && below.high < 0.02, "{below:?}");
        assert!(above.high > 0.98 && above.low < 0.02, "{above:?}");
        // Butterworth: 3 dB down at the cutoff, either way.
        assert!((at.low - 0.707).abs() < 0.01, "{at:?}");
        assert!((at.high - 0.707).abs() < 0.01, "{at:?}");
        assert!(at.band > below.band && at.band > above.band);
    }

    #[test]
    fn stays_finite_swept_hard_with_high_resonance() {
        let mut svf = Svf::new(44_100.0);
        let mut seed = 1u32;
        let mut worst = 0.0f32;
        for i in 0..44_100 {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let x = seed as f32 / u32::MAX as f32 - 0.5;
            svf.set(20.0 + 20_000.0 * ((i % 97) as f32 / 97.0), 20.0);
            let out = svf.next(x);
            worst = worst.max(out.low.abs()).max(out.high.abs());
        }

        assert!(worst.is_finite() && worst < 100.0, "{worst}");
        svf.reset();
        assert!(svf.next(0.0).low.abs() < 1e-12);
    }
}
