//! What every plugin in this repo is made of: a spec (who it is, its
//! parameters and meters), saved state, and one `process` call. The
//! format crates (`kit-vst3`, `kit-clap`) turn a `Plugin` into something
//! a host loads; nothing here knows about a host.

mod learn;
pub mod level;
mod param;
mod plugin;
mod runner;
mod scope;
mod shared;
pub mod state;

pub use learn::{LEARNABLE, Learn};
pub use param::{
    BYPASS, Kind, MATCH, MIX, Param, balance, bypass, iron_core, iron_grip, iron_roll, iron_weight,
    level_match, list, listen, meter, mix, range, toggle, tube_bias, tube_glow, tube_grid,
    tube_sag,
};
pub use plugin::{Asset, Info, Plugin, Role, Spec};
pub use runner::{Block, EVENTS, Event, Runner, Transport};
pub use scope::{SCOPE, Scope};
pub use shared::{BEGIN, END, SET, Shared};
