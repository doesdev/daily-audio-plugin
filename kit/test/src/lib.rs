//! What every plugin in this repo is held to. A plugin's
//! `tests/suite.rs` is one line, `kit_test::suite!(Mine);`, and gets a
//! test for each check in `checks`. Its own behaviour it tests itself,
//! with the signals and the WAV writer here.

pub mod budget;
pub mod checks;
pub mod signals;
pub mod wav;

pub use budget::{Counting, allocations};

/// The checks every plugin must pass, as tests, for plugin `$plugin`.
/// Sets the test binary's allocator to one that counts, so it belongs
/// in a test file of its own.
#[macro_export]
macro_rules! suite {
    ($plugin:ty) => {
        #[global_allocator]
        static ALLOC: $crate::Counting = $crate::Counting;

        #[test]
        fn spec_is_sound() {
            $crate::checks::spec_is_sound::<$plugin>();
        }

        #[test]
        fn output_is_finite() {
            $crate::checks::output_is_finite::<$plugin>();
        }

        #[test]
        fn sounds_at_its_defaults() {
            $crate::checks::sounds_at_its_defaults::<$plugin>();
        }

        #[test]
        fn is_deterministic() {
            $crate::checks::is_deterministic::<$plugin>();
        }

        #[test]
        fn block_size_is_not_heard() {
            $crate::checks::block_size_is_not_heard::<$plugin>();
        }

        #[test]
        fn never_allocates() {
            $crate::checks::never_allocates::<$plugin>();
        }

        #[test]
        fn is_within_budget() {
            $crate::checks::is_within_budget::<$plugin>();
        }

        #[test]
        fn a_tail_costs_no_more_than_the_sound() {
            $crate::checks::a_tail_costs_no_more_than_the_sound::<$plugin>();
        }

        #[test]
        fn loads_in_every_format() {
            $crate::checks::loads_in_every_format::<$plugin>();
        }
    };
}
