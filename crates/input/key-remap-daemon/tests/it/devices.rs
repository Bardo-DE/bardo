//! The real kernel path: the built daemon grabbing a fake keyboard on `/dev/uinput`, remapping
//! and writing "key-remap keyboard", read back from `/dev/input`.
//!
//! These create input devices, so each is `#[ignore]`d and asserts `BARDO_KEY_REMAP_DEVICE_TESTS=1`;
//! CI runs them as root with `-- --ignored --test-threads=1` (one keyboard name, one daemon at
//! a time, filtered to `devices::`). Only `key_table` runs in a plain `cargo test`.

mod conflict;
mod key_table;
mod keymap;
mod leds;
mod lifecycle;
mod mapping;
mod pause;
mod seat;
mod support;
mod watchdog;
