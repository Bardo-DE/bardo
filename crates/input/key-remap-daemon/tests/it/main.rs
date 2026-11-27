//! The integration tests, one binary: whole-core scenarios, the terminal and Plasma profiles, the
//! stuck-key property test, and `live`, the real kernel path (it creates real input devices; see
//! `live.rs`).

mod key_table;
mod keymap_aware;
mod live;
mod plasma_overlay;
mod scenarios;
mod stuck_keys;
mod support;
mod terminal;
