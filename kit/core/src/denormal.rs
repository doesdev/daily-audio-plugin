//! Numbers too small to matter, kept from costing anything.
//!
//! A sound that dies away does not reach zero: a filter's memory, a
//! follower, a string's last ringing all shrink toward it for ever,
//! and below about 1e-38 a float is denormal. A processor works on
//! those many times more slowly, so a plugin that has gone quiet costs
//! more than one that is playing: measured here, an instrument's tail
//! took eight times what its notes did, for as long as it was left.
//!
//! The cure is the processor's own: two switches that treat such
//! numbers as zero, going in and coming out. They are set for the
//! length of a block and put back after, since the thread is the
//! host's and its settings are not ours to leave changed. Nothing
//! audible is lost: these are 760 dB below full scale.

/// While this lives, the thread's arithmetic takes denormals as zero.
/// Dropping it puts the thread's own setting back.
pub struct Flushed(#[allow(dead_code)] u64);

impl Flushed {
    #[inline]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(set(None))
    }
}

impl Drop for Flushed {
    #[inline]
    fn drop(&mut self) {
        set(Some(self.0));
    }
}

// Sets the two switches, or puts back `to`: what was there before.
#[cfg(target_arch = "x86_64")]
#[inline]
fn set(to: Option<u64>) -> u64 {
    // Flush to zero (bit 15) and denormals are zero (bit 6).
    const BOTH: u32 = 0x8040;
    let mut was = 0u32;
    unsafe {
        std::arch::asm!("stmxcsr dword ptr [{}]", in(reg) &mut was, options(nostack));
        let now = match to {
            Some(old) => old as u32,
            None => was | BOTH,
        };
        std::arch::asm!("ldmxcsr dword ptr [{}]", in(reg) &now, options(nostack));
    }
    u64::from(was)
}

#[cfg(target_arch = "aarch64")]
#[inline]
fn set(to: Option<u64>) -> u64 {
    // Flush to zero is bit 24 of the control register.
    const FZ: u64 = 1 << 24;
    let was: u64;
    unsafe {
        std::arch::asm!("mrs {}, fpcr", out(reg) was, options(nomem, nostack));
        let now = to.unwrap_or(was | FZ);
        std::arch::asm!("msr fpcr, {}", in(reg) now, options(nomem, nostack));
    }
    was
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline]
fn set(_to: Option<u64>) -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hint::black_box;

    // The smallest ordinary float, halved where the compiler cannot
    // do it ahead of time: denormal, unless they are being flushed.
    fn halved() -> f32 {
        black_box(f32::MIN_POSITIVE) * black_box(0.5)
    }

    #[test]
    fn denormals_are_zero_while_flushed_and_themselves_again_after() {
        assert!(halved() > 0.0);
        {
            let _flushed = Flushed::new();
            assert_eq!(halved(), 0.0);
            // And one handed in is taken as zero too.
            let tiny = black_box(f32::from_bits(1));
            assert_eq!(tiny * black_box(1.0e30), 0.0);
        }
        assert!(halved() > 0.0, "the thread's own setting is put back");
    }

    #[test]
    fn one_inside_another_leaves_the_outer_in_force() {
        let _outer = Flushed::new();
        {
            let _inner = Flushed::new();
        }
        assert_eq!(halved(), 0.0);
    }
}
