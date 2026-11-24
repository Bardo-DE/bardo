#![allow(
    dead_code,
    reason = "each test binary of the crate uses a subset of the shared helpers"
)]


/// intentd's scripted inferd, included once for the whole executable.
#[path = "../../../action-daemon/tests/it/support/inferd.rs"]
pub mod inferd;
