//! How loud a signal is right now: its level, rising at one speed and
//! falling at another. What a compressor, a gate or a meter listens to.

use crate::{TINY, pole};

#[derive(Debug, Clone, Copy)]
pub struct Follower {
    level: f32,
    attack: f32,
    release: f32,
}

impl Follower {
    pub fn new(attack_ms: f32, release_ms: f32, rate: f32) -> Self {
        Self {
            level: 0.0,
            attack: pole(attack_ms, rate),
            release: pole(release_ms, rate),
        }
    }

    pub fn reset(&mut self) {
        self.level = 0.0;
    }

    pub fn get(&self) -> f32 {
        self.level
    }

    #[inline]
    pub fn next(&mut self, x: f32) -> f32 {
        let x = x.abs();
        let pole = if x > self.level {
            self.attack
        } else {
            self.release
        };
        self.level = x + (self.level - x) * pole + TINY;
        self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rises_fast_and_falls_slow() {
        let rate = 48_000.0;
        let mut f = Follower::new(1.0, 100.0, rate);
        let ms = |n: f32| (rate * n / 1000.0) as usize;
        for _ in 0..ms(5.0) {
            f.next(-0.8);
        }
        let risen = f.get();
        for _ in 0..ms(5.0) {
            f.next(0.0);
        }
        let held = f.get();
        for _ in 0..ms(500.0) {
            f.next(0.0);
        }

        assert!(risen > 0.79, "{risen}");
        assert!(held > 0.7, "still most of the way up: {held}");
        assert!(f.get() < 0.01, "{}", f.get());
    }
}
