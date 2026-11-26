//! Integration tests of `memory-daemon`.
mod binary;
mod bus;
mod bus_members;
mod desktop;
mod inferd_bus;
mod keyring;
mod peers;
mod persistence;
mod queue;
mod removals;
mod restart;
mod sandbox;
mod settings;
mod support;
// The wire samples live in memory-core's test support.
#[path = "../../../memory-core/tests/it/support.rs"]
mod wire;
mod xdg;
