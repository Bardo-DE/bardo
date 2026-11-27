//! The real kernel path: the built daemon grabbing a fake keyboard on `/dev/uinput`, remapping
//! and writing "key-remap keyboard", read back from `/dev/input`.
//!
//! These create input devices, so they are the `live` suite (`cargo xtask live devices`), run as
//! a user that may open `/dev/uinput`, one at a time (one keyboard name, one daemon at a time).

mod conflict;
mod keymap;
mod leds;
mod lifecycle;
mod mapping;
mod pause;
mod seat;
mod support;
mod watchdog;
