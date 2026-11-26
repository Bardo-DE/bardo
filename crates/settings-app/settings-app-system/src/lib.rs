//! The real services the Settings app reaches, and their fakes: the bus, the portal,
//! NetworkManager, BlueZ, the compositor, accountd, inferd, the actions service, memory and syncd. One module
//! per service, named as its `settings-app-link` trait is, each with the real implementation
//! (zbus proxies, wire decoding, error to `Absence`) and an in-memory fake under the `testing`
//! feature. Pure rules found while reading the wire live in `settings-app-model`; nothing here
//! draws.
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

pub mod accounts;
pub mod bluetooth;
pub mod bus;
pub mod chooser;
pub mod computers;
pub mod displays;
pub mod engine_install;
pub mod engines;
#[cfg(feature = "testing")]
mod fake_kit;
pub mod hot_corners;
pub mod lending;
#[cfg(feature = "testing")]
mod lock;
pub mod memory_consent;
pub mod modules;
pub mod person;
pub mod picker;
pub mod privacy;
pub mod running;
pub mod screen_sharing;
pub mod services;
pub mod spaces;
pub mod sync;
pub mod updates;
pub mod wallpaper;
pub mod wifi;
