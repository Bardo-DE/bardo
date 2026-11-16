//! The Linux desktop's own services: the D-Bus single instance and the agent intents. Built with
//! the `quire-desktop` feature on Linux. A new platform adds a sibling module with the same shapes
//! and selects it in `lib.rs`.

mod env;
mod instance;
mod intents;

/// The public path of the Linux desktop's implementations, `anyview_platform::linux`.
pub mod linux {
    pub use super::instance::{BUS_NAME, DbusInstance, forward_over};
    pub use super::intents::{AGENT_BUS_NAME, OPEN_FILES};
    pub use crate::portable::FreedesktopThumbnails;
}
