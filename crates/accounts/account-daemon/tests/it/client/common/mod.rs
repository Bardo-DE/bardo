//! Shared by the bus tests: a private bus and the real accountd on it.
#![allow(
    dead_code,
    reason = "helpers shared by the test files; each file uses some of them"
)]

pub mod accountd;
pub mod bus;
pub mod fake_accountd;

pub use base_testkit::eventually;
