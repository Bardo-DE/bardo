//! The kernel's side: `/dev/uinput`. [`real`] is the one implementation of
//! `input_broker_core::backend::Backend` that touches a device; `fake` records instead, so the
//! service is tested without one.

// Test seams: never in a shipped build.
#[cfg(any(test, feature = "fake"))]
pub mod fake;
pub mod real;
