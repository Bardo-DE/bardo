//! The integration tests, one binary: whole-core scenarios, the terminal and Plasma profiles, and
//! the stuck-key property test, and `devices`, the real kernel path (it creates real input
//! devices, so each of its tests is `#[ignore]`d; see `devices.rs`).

mod devices;
mod keymap_aware;
mod plasma_overlay;
mod scenarios;
mod stuck_keys;
mod support;
mod terminal;
