//! A setting eased toward where it was put, so a knob turned during
//! playback does not click. One per setting that scales the signal.

use crate::pole;

#[derive(Debug, Clone, Copy)]
pub struct Smooth {
    now: f32,
    pole: f32,
}

impl Smooth {
    /// Starts at `value` and covers most of any later move in `ms`.
    pub fn new(value: f32, ms: f32, rate: f32) -> Self {
        Self {
            now: value,
            pole: pole(ms, rate),
        }
    }

    /// Jumps to `value` with no easing: for a reset.
    pub fn snap(&mut self, value: f32) {
        self.now = value;
    }

    pub fn get(&self) -> f32 {
        self.now
    }

    /// One sample nearer `target`.
    #[inline]
    pub fn next(&mut self, target: f32) -> f32 {
        self.now = target + (self.now - target) * self.pole;
        self.now
    }

    /// Whether it has as good as arrived, and a block can skip easing.
    pub fn settled(&self, target: f32) -> bool {
        (self.now - target).abs() < 1e-4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_most_of_a_step_in_its_time_and_never_overshoots() {
        let rate = 48_000.0;
        let mut s = Smooth::new(0.0, 10.0, rate);
        let mut last = 0.0;
        for _ in 0..(rate * 0.010) as usize {
            let v = s.next(1.0);
            assert!(v >= last && v <= 1.0, "{v} after {last}");
            last = v;
        }

        assert!((last - 0.632).abs() < 0.01, "{last}");
        for _ in 0..(rate * 0.2) as usize {
            s.next(1.0);
        }
        assert!(s.settled(1.0), "{}", s.get());
    }

    #[test]
    fn no_time_is_no_easing() {
        let mut s = Smooth::new(0.0, 0.0, 48_000.0);

        assert_eq!(s.next(0.7), 0.7);
        s.snap(0.2);
        assert_eq!(s.get(), 0.2);
    }
}
