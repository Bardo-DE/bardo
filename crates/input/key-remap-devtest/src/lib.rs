//! The harness the device tests drive the key remap daemon with: a fake keyboard on `/dev/uinput`, the real
//! daemon binary as a child process, the "key-remap keyboard" it writes, its socket and its
//! `sd_notify` messages.
//!
//! Everything here creates or opens real input devices. Nothing in the workspace's ordinary
//! tests may call it: the device tests that do live in the `live` module of the key remap
//! daemon's integration binary (`cargo xtask live devices`).
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

mod client;
mod daemon;
mod event;
mod failure;
mod fake;
mod grab;
mod keys;
mod notify;
mod output;
mod sysfs;

pub use client::{Client, FOCUS_SETTLE, summary};
pub use daemon::{Daemon, DaemonSpec};
pub use event::{EV_ABS, EV_KEY, EV_LED, EV_REL, EV_SYN, Event, LED_CAPSL, SYN_REPORT};
pub use failure::DevtestError;
pub use fake::{FakeKeyboard, FakeSpec, Pointer};
pub use grab::ExternalGrab;
pub use keys::{KEYS, code_of, name_of};
pub use notify::{Note, Notifications};
pub use output::Output;
pub use sysfs::OUTPUT_NAME;
