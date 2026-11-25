//! The sound itself, for the editor to draw: the last couple of
//! thousand samples that came in and that went out, each mixed to one
//! channel. The audio thread writes and never waits; whoever draws
//! reads whenever it likes. A read may straddle a write by a sample or
//! two, which a picture does not show.

use crate::Learn;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// How many samples are kept each way: enough for a wave on the glass
/// and for a spectrum of a thousand bins.
pub const SCOPE: usize = 2048;

pub struct Scope {
    /// Which controller moves which setting. It is here because this
    /// is the one thing the audio thread and the editor both hold in
    /// every format: see `kit-vst3`'s `connect`.
    pub learn: Learn,
    input: Box<[AtomicU32]>,
    output: Box<[AtomicU32]>,
    written: AtomicU64,
    rate: AtomicU32,
}

impl Default for Scope {
    fn default() -> Self {
        let ring = || (0..SCOPE).map(|_| AtomicU32::new(0)).collect();
        Self {
            learn: Learn::default(),
            input: ring(),
            output: ring(),
            written: AtomicU64::new(0),
            rate: AtomicU32::new(0),
        }
    }
}

impl Scope {
    pub fn set_rate(&self, rate: f32) {
        self.rate.store(rate.to_bits(), Ordering::Relaxed);
    }

    /// The sample rate of what is kept, or 0 before audio has run.
    pub fn rate(&self) -> f32 {
        f32::from_bits(self.rate.load(Ordering::Relaxed))
    }

    /// From the audio thread: one sample each way. Never allocates,
    /// locks or waits.
    pub fn push(&self, input: f32, output: f32) {
        let at = self.written.load(Ordering::Relaxed);
        let slot = (at % SCOPE as u64) as usize;
        self.input[slot].store(input.to_bits(), Ordering::Relaxed);
        self.output[slot].store(output.to_bits(), Ordering::Relaxed);
        self.written.store(at + 1, Ordering::Release);
    }

    /// How many samples have ever been written: it moves while audio
    /// runs, so a reader can tell there is something new.
    pub fn written(&self) -> u64 {
        self.written.load(Ordering::Acquire)
    }

    /// The last `n` samples each way, oldest first, into `input` and
    /// `output`. Fewer if fewer have been written. Not for the audio
    /// thread: it may grow the vectors.
    pub fn latest(&self, n: usize, input: &mut Vec<f32>, output: &mut Vec<f32>) {
        let end = self.written();
        let n = n.min(SCOPE).min(end as usize);
        let read = |ring: &[AtomicU32], into: &mut Vec<f32>| {
            into.clear();
            into.extend((end - n as u64..end).map(|at| {
                f32::from_bits(ring[(at % SCOPE as u64) as usize].load(Ordering::Relaxed))
            }));
        };
        read(&self.input, input);
        read(&self.output, output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn latest(scope: &Scope, n: usize) -> (Vec<f32>, Vec<f32>) {
        let (mut input, mut output) = (Vec::new(), Vec::new());
        scope.latest(n, &mut input, &mut output);
        (input, output)
    }

    #[test]
    fn the_latest_samples_come_back_oldest_first() {
        let scope = Scope::default();
        (0..5).for_each(|i| scope.push(i as f32, -(i as f32)));

        assert_eq!(scope.written(), 5);
        assert_eq!(
            latest(&scope, 3),
            (vec![2.0, 3.0, 4.0], vec![-2.0, -3.0, -4.0])
        );
        // No more than there is.
        assert_eq!(latest(&scope, 99).0, [0.0, 1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn it_keeps_only_the_newest_once_it_is_full() {
        let scope = Scope::default();
        let total = SCOPE + 10;
        (0..total).for_each(|i| scope.push(i as f32, 0.0));

        let (input, _) = latest(&scope, SCOPE + 500);
        assert_eq!(input.len(), SCOPE);
        assert_eq!(input[0], 10.0);
        assert_eq!(input[SCOPE - 1], (total - 1) as f32);
    }

    #[test]
    fn nothing_written_is_nothing_read_and_the_rate_is_kept() {
        let scope = Scope::default();

        assert_eq!(latest(&scope, 8), (vec![], vec![]));
        assert_eq!(scope.rate(), 0.0);
        scope.set_rate(44_100.0);
        assert_eq!(scope.rate(), 44_100.0);
    }
}
