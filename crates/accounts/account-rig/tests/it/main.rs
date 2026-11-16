//! The crate's integration tests: one executable, one module per topic.
//! `secrets` starts this test executable again as a fake child, by test path
//! (`secrets::oo7_child`).

mod client;
mod common;
mod google_levers;
mod secrets;
mod servers;
