//! Shared by the bus tests: a private bus, a fake inferd on it, and the real session server over
//! scripted seams.
#![allow(
    dead_code,
    reason = "each test binary uses a different part of this shared module"
)]

pub mod bus;
pub mod inferd;
pub mod served;
