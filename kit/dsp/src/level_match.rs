//! Level matching: a gain that holds what goes out as loud as what
//! came in, so a change of setting is heard as a change of tone and
//! not as "louder, so better". It compares the two over a few tenths
//! of a second, never moves fast enough to act as a compressor, and
//! holds still through silence.

use crate::pole;

/// How long the two levels are averaged over.
const WINDOW_MS: f32 = 400.0;
/// How quickly the gain moves to where it should be.
const EASE_MS: f32 = 150.0;
/// The most it will turn anything up or down: 24 dB.
const REACH: f32 = 15.85;
/// Mean square below which there is nothing to match: -80 dB.
const FLOOR: f32 = 1e-8;

#[derive(Debug, Clone, Copy)]
pub struct Match {
    heard: f32,
    made: f32,
    target: f32,
    gain: f32,
    window: f32,
    ease: f32,
}

impl Match {
    pub fn new(rate: f32) -> Self {
        Self {
            heard: 0.0,
            made: 0.0,
            target: 1.0,
            gain: 1.0,
            window: pole(WINDOW_MS, rate),
            ease: pole(EASE_MS, rate),
        }
    }

    pub fn reset(&mut self) {
        (self.heard, self.made, self.target, self.gain) = (0.0, 0.0, 1.0, 1.0);
    }

    /// The gain it is applying now.
    pub fn gain(&self) -> f32 {
        self.gain
    }

    /// One frame: `heard` is the square of what came in and `made` the
    /// square of what would go out, each summed over the channels.
    /// The gain to multiply what goes out by. It listens whether or
    /// not it is `on`, so switching it on does not start from nothing;
    /// off, the gain eases back to one.
    #[inline]
    pub fn next(&mut self, heard: f32, made: f32, on: bool) -> f32 {
        self.heard = heard + (self.heard - heard) * self.window;
        self.made = made + (self.made - made) * self.window;
        // With nothing coming in or nothing going out there is nothing
        // to compare: the last answer stands.
        if self.heard > FLOOR && self.made > FLOOR {
            self.target = (self.heard / self.made).sqrt().clamp(1.0 / REACH, REACH);
        }
        let to = if on { self.target } else { 1.0 };
        self.gain = to + (self.gain - to) * self.ease;
        self.gain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    // Runs a sine of `level` in and `ratio` times that out for
    // `seconds`, and gives the gain at the end.
    fn settle(matcher: &mut Match, level: f32, ratio: f32, seconds: f32, on: bool) -> f32 {
        (0..(RATE * seconds) as usize).fold(1.0, |_, i| {
            let x = level * (i as f32 * 0.05).sin();
            matcher.next(x * x, x * x * ratio * ratio, on)
        })
    }

    #[test]
    fn it_undoes_a_change_of_level_whichever_way_it_went() {
        let cases = [(2.0, 0.5), (0.25, 4.0), (1.0, 1.0), (4.0, 0.25)];

        for (ratio, want) in cases {
            let gain = settle(&mut Match::new(RATE), 0.3, ratio, 3.0, true);
            assert!((gain - want).abs() < want * 0.01, "{ratio}: {gain}");
        }
    }

    #[test]
    fn it_holds_through_silence_and_does_not_chase_noise() {
        let mut matcher = Match::new(RATE);
        let before = settle(&mut matcher, 0.3, 2.0, 3.0, true);
        let after = settle(&mut matcher, 0.0, 2.0, 3.0, true);

        assert!((after - before).abs() < 0.01, "{before} then {after}");
    }

    #[test]
    fn it_never_reaches_past_24_db() {
        let up = settle(&mut Match::new(RATE), 0.3, 0.001, 3.0, true);
        let down = settle(&mut Match::new(RATE), 0.001, 1000.0, 3.0, true);

        assert!((up - REACH).abs() < 0.2, "{up}");
        assert!((down - 1.0 / REACH).abs() < 0.01, "{down}");
    }

    #[test]
    fn off_it_listens_but_leaves_the_level_alone_and_on_it_is_ready() {
        let mut matcher = Match::new(RATE);
        let off = settle(&mut matcher, 0.3, 2.0, 3.0, false);
        assert!((off - 1.0).abs() < 1e-6, "{off}");

        // Already knows the answer: it is there within its ease.
        let on = settle(&mut matcher, 0.3, 2.0, 1.0, true);
        assert!((on - 0.5).abs() < 0.01, "{on}");
    }

    #[test]
    fn it_moves_too_slowly_to_be_a_compressor() {
        let mut matcher = Match::new(RATE);
        settle(&mut matcher, 0.3, 1.0, 2.0, true);
        // A sudden doubling: a millisecond later it has barely moved.
        let soon = settle(&mut matcher, 0.3, 2.0, 0.001, true);

        assert!(soon > 0.99, "{soon}");
    }
}
