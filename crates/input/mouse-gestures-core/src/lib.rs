//! The mouse gestures daemon without its devices: the Magic Mouse gesture daemon's decisions
//! (12-BEHAVIOUR-gestures).
//!
//! Everything here takes bytes and instants and returns values: the report decoders (`sensor`),
//! `touch`, `stamp`, the scroll `feel`, `gestures`, the `tracker` that holds the whole gesture
//! state, `action`, the settings (`config`: sections, units, the layering of the machine's and the
//! person's files, the schema), the `outputs` seam, and `socket`, the decisions the daemon's
//! socket thread carries out. `recording` and `replay` feed recorded reads through it. No file is
//! read, no thread started, no clock read and no device opened here: the daemon owns the hidraw,
//! evdev, uinput and socket I/O, and `command::run` there is the one way in to it. The socket
//! messages are the `mouse-gestures-proto` crate.
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

pub mod action;
pub mod config;
pub mod feel;
pub mod gestures;
pub mod keys;
pub mod outputs;
pub mod recording;
pub mod replay;
pub mod sensor;
pub mod socket;
pub mod stamp;
pub mod time;
pub mod touch;
pub mod tracker;

pub use config::Config;
pub use sensor::{Frame, Sensor, Surface};
pub use time::Mono;
pub use tracker::{Out, Tracker};
