//! The mouse gestures daemon: the Magic Mouse gesture daemon (12-BEHAVIOUR-gestures).
//!
//! The pure core (`sensor`, `touch`, `stamp`, `feel`, `gestures`, `tracker`, `action`,
//! `config`, `outputs`) takes bytes and instants and returns values; `recording` and `replay`
//! feed recorded reads through it. The daemon owns every thread, socket, device and clock, and
//! it holds no `unsafe` (the hidraw ioctl is the `hidraw-ioctl` crate); `command::run` is the one way in to it. The socket
//! messages are the `mouse-gestures-proto` crate.
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

pub mod action;
pub mod command;
pub mod config;
pub(crate) mod daemon;
pub mod error;
pub mod feel;
pub mod gestures;
pub mod keys;
pub mod outputs;
pub mod recording;
pub mod replay;
pub mod sensor;
pub mod stamp;
pub mod time;
pub mod touch;
pub mod tracker;

pub use config::Config;
pub use sensor::{Frame, Sensor, Surface};
pub use time::Mono;
pub use tracker::{Out, Tracker};
