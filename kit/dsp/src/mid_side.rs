//! Hearing and weighing the middle and the sides: every plugin that
//! works on them apart lets a person solo either and turn one down
//! against the other, the same way everywhere.

use crate::Smooth;

/// Which of the two is heard.
pub const BOTH: usize = 0;
pub const MID: usize = 1;
pub const SIDE: usize = 2;

/// The gains for the middle and the sides at `balance`, from -1 (the
/// middle alone) through 0 (both, untouched) to 1 (the sides alone),
/// hearing `listen`. One control turns the other half down; neither is
/// ever turned up, so the output level is still the output's to set.
pub fn mid_side_gains(balance: f32, listen: usize) -> (f32, f32) {
    let b = balance.clamp(-1.0, 1.0);
    // Squared, so the first half of the travel is gentle.
    let down = |by: f32| (1.0 - by.max(0.0)).powi(2);
    let (mid, side) = (down(b), down(-b));
    match listen {
        MID => (mid, 0.0),
        SIDE => (0.0, side),
        _ => (mid, side),
    }
}

/// The same, eased so a solo or a move of the balance does not click.
#[derive(Debug, Clone, Copy)]
pub struct MidSide {
    mid: Smooth,
    side: Smooth,
}

impl MidSide {
    pub fn new(rate: f32) -> Self {
        Self {
            mid: Smooth::new(1.0, 20.0, rate),
            side: Smooth::new(1.0, 20.0, rate),
        }
    }

    /// One frame's gains for the middle and the sides.
    #[inline]
    pub fn next(&mut self, balance: f32, listen: usize) -> (f32, f32) {
        let (mid, side) = mid_side_gains(balance, listen);
        (self.mid.next(mid), self.side.next(side))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_balance_turns_one_down_and_never_the_other_up() {
        let cases = [
            (0.0, BOTH, (1.0, 1.0)),
            (1.0, BOTH, (0.0, 1.0)),
            (-1.0, BOTH, (1.0, 0.0)),
            (0.5, BOTH, (0.25, 1.0)),
            (-0.5, BOTH, (1.0, 0.25)),
            (0.0, MID, (1.0, 0.0)),
            (0.0, SIDE, (0.0, 1.0)),
            (0.5, MID, (0.25, 0.0)),
            (9.0, BOTH, (0.0, 1.0)),
        ];

        for (balance, listen, want) in cases {
            assert_eq!(mid_side_gains(balance, listen), want, "{balance} {listen}");
        }
    }

    #[test]
    fn a_solo_comes_in_without_a_step() {
        let mut eased = MidSide::new(48_000.0);
        let first = eased.next(0.0, MID);
        let later = (0..9600).fold((0.0, 0.0), |_, _| eased.next(0.0, MID));

        assert!(first.1 > 0.99, "{first:?}");
        assert!(later.1 < 0.001 && later.0 > 0.999, "{later:?}");
    }
}
