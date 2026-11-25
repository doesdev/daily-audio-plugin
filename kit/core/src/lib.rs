//! What every plugin in this repo is made of: a spec (who it is, its
//! parameters and meters), saved state, and one `process` call. The
//! format crates (`kit-vst3`, `kit-clap`) turn a `Plugin` into something
//! a host loads; nothing here knows about a host.

pub mod level;
mod param;
mod plugin;
mod runner;
mod shared;
pub mod state;

pub use param::{Kind, Param, list, meter, range, toggle};
pub use plugin::{Asset, Info, Plugin, Role, Spec};
pub use runner::{Block, EVENTS, Event, Runner, Transport};
pub use shared::{BEGIN, END, SET, Shared};
