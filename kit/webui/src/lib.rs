//! A plugin's editor: the kit's page (`kit/ui`) and the plugin's own
//! files beside it, in the system webview, as a child of the window a
//! host hands over. The page is plain ES modules served from the
//! binary; `bridge` is everything the page and the plugin say to each
//! other. Windows only for now; elsewhere there is no editor.

pub mod bridge;
pub mod docs;
#[cfg(windows)]
mod panel;
pub mod presets;
#[cfg(windows)]
#[doc(hidden)]
pub mod testing;
#[cfg(windows)]
mod window;

pub use bridge::Action;
#[cfg(windows)]
pub use panel::{OnAction, Panel, Pull, Values};
/// Call when the DLL holding panels unloads.
#[cfg(windows)]
pub use window::{Hwnd, unregister};

/// Call when the DLL holding panels unloads.
#[cfg(not(windows))]
pub fn unregister() {}
