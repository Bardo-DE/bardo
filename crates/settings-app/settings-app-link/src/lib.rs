//! What the Settings app asks of the computer, and no more: one `Link` trait per service (the
//! bus, the portal, NetworkManager, BlueZ, the compositor, accountd, inferd, docket, memory and
//! syncd), the facts they take and return (`settings-app-model`'s), the deadlines every call runs
//! under and the ticks that say "look again".
//!
//! The client holds links and never names how they are reached; `settings-app-system` is the real
//! implementation of every one (zbus proxies, wire decoding, error to `Absence`) and the in-memory
//! fakes for tests. Nothing here talks to a service.

pub mod accounts;
pub mod bluetooth;
pub mod bus;
pub mod chooser;
pub mod computers;
mod deadline;
#[cfg(test)]
mod deadline_tests;
pub mod displays;
pub mod engine_install;
pub mod engines;
pub mod hot_corners;
pub mod lending;
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
mod ticks;
#[cfg(test)]
mod ticks_tests;
pub mod updates;
pub mod wallpaper;
pub mod wifi;

pub use deadline::{Deadline, answered, peer};
pub use ticks::{Gone, Ticker, Ticks};
