//! The mouse gestures daemon: the Magic Mouse gesture daemon (12-BEHAVIOUR-gestures).
//!
//! The pure core (`sensor`, `touch`, `stamp`, `feel`, `gestures`, `tracker`, `action`, `config`'s
//! settings, `outputs`, and the decisions of the socket) is `mouse-gestures-core`: it takes bytes
//! and instants and returns values. This crate owns every thread, socket, device and clock, and it
//! holds no `unsafe` (the hidraw ioctl is the `hidraw-ioctl` crate); `command::run` is the one way
//! in to it. The socket messages are the `mouse-gestures-proto` crate.
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

pub mod command;
pub mod config;
pub(crate) mod daemon;
pub mod error;
