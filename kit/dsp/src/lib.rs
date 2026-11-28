//! Building blocks for the audio thread. Each is made (and may
//! allocate) in a plugin's `prepare`, and from then on only does
//! arithmetic: no allocation, no locks, no waiting.
//!
//! The kit ratchets: when a plugin needs a block that is not here and
//! another plugin could use, it goes in here with its own tests, not
//! in the plugin.

mod delay;
mod follower;
mod level_match;
mod mid_side;
mod oversample;
mod smooth;
mod svf;
mod transformer;
mod tube;

pub use delay::Delay;
pub use follower::Follower;
pub use level_match::Match;
pub use mid_side::{BOTH, MID, MidSide, SIDE, mid_side_gains};
pub use oversample::{Bend, FLOOR, MOST, Need, Oversampled, dot};
pub use smooth::Smooth;
pub use svf::{Svf, SvfOut};
pub use transformer::{IronShape, PUSH, Transformer, iron_curve};
pub use tube::{
    PENTODE, PLAYING_TURNS, RENDERING_TURNS, STRAPPED, TRIODE, Tube, TubeShape, tube_curve,
};

/// A number so small it is silence, added and taken away again so a
/// decaying filter never slows to a crawl on denormals.
pub const TINY: f32 = 1e-20;

/// The one-pole coefficient that covers 63% of a step in `ms`.
pub fn pole(ms: f32, rate: f32) -> f32 {
    match ms > 0.0 {
        true => (-1.0 / (ms * 0.001 * rate)).exp(),
        false => 0.0,
    }
}
