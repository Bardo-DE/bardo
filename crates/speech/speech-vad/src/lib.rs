//! Voice activity without a model: an energy gate (the test default), a framer that cuts audio
//! chunks into 512-sample frames, a level meter for the orb, and the pure endpointing machine.
//!
//! Everything is synchronous and does no I/O. Time is an input: the position in samples.
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

mod endpoint;
mod energy;
mod framer;
mod level;

pub use endpoint::{EndWhy, Endpoint, EndpointParams, endpoint};
pub use energy::{EnergyGate, EnergyGateParams, FrameCount};
pub use framer::{FramedAt, Framer, FramerError};
pub use level::{Level, level_of};
