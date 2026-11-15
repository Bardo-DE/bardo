//! The in-process integration tests, one binary: whole-core scenarios, the terminal and Plasma
//! profiles, and the stuck-key property test. `tests/devices` stays its own binary (it creates
//! real input devices, needs root and `--test-threads=1`).

mod keymap_aware;
mod plasma_overlay;
mod scenarios;
mod stuck_keys;
mod support;
mod terminal;
