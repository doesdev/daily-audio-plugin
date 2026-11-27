//! What every plugin in this repo is made of: a spec (who it is, its
//! parameters and meters), saved state, and one `process` call. The
//! format crates (`kit-vst3`, `kit-clap`) turn a `Plugin` into something
//! a host loads; nothing here knows about a host.

/// The brand every plugin here ships under: Daily Audio Plugin, DAP
/// for short. `VENDOR` is what a host shows as the maker.
pub const VENDOR: &str = "Daily Audio Plugin";
pub const BRAND: &str = "DAP";
/// The folder a person's presets and the editor's own data are kept
/// under. It is the name the first plugins shipped with and stays so:
/// renaming it would strand what people have already saved there.
pub const HOME: &str = "doesdev";

mod learn;
pub mod level;
pub mod link;
mod param;
mod plugin;
mod runner;
mod scope;
mod shared;
pub mod state;

pub use learn::{LEARNABLE, Learn};
pub use link::{Links, Peer};
pub use param::{
    BYPASS, Kind, MATCH, MIX, Param, balance, bypass, iron_drive, iron_grip, iron_material,
    iron_ring, iron_size, iron_source, iron_top, key, level_match, list, listen, meter, mix, range,
    toggle, tube_bias, tube_bloom, tube_feedback, tube_glow, tube_grid, tube_sag, tube_stage,
};
pub use plugin::{Asset, Info, Plugin, Role, Spec};
pub use runner::{Block, EVENTS, Event, Runner, Transport, source};
pub use scope::{SCOPE, Scope};
pub use shared::{BEGIN, END, SET, Shared};
