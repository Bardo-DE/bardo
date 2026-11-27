//! The crate's integration tests: one executable, one module per topic.
//! `accept::secrets` starts this test executable again as a fake child, by test path
//! (`accept::secrets::oo7_child`).

mod accept;
mod common;
mod google_levers;
