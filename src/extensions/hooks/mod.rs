//! Hook implementations and helpers for adding to your Penrose window manager
pub mod ewmh;
pub mod manage;
pub mod mejo13;
pub mod named_scratchpads;
pub mod startup;
pub mod window_swallowing;

pub use ewmh::add_ewmh_hooks;
pub use mejo13::ClientPositioning;
pub use named_scratchpads::{add_named_scratchpads, NamedScratchPad, ToggleNamedScratchPad};
pub use startup::SpawnOnStartup;
pub use window_swallowing::WindowSwallowing;
