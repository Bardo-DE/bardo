//! What every platform has: no D-Bus and no freedesktop service is called from here.
//!
//! Single instance over the per-user socket (`LatchkeyInstance`) and the shared thumbnail cache (a
//! file layout). `linux` (feature `quire-desktop`) replaces the single instance with the desktop's
//! own.

mod frame;
mod instance;
mod thumbnails;

pub use instance::LatchkeyInstance;
pub use thumbnails::FreedesktopThumbnails;
