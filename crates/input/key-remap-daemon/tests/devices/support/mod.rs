//! The device tests' harness: a scene (scratch directory, failure report) and the usual rig.

#![allow(
    dead_code,
    reason = "shared test helpers: each test file uses only some of them"
)]

mod rig;
mod scene;

pub use rig::{GRAB_WITHIN, Rig};
pub use scene::{FAKE_NAME, Scene, WATCHDOG, ok};
