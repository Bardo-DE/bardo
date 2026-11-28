//! The `PipeWire` side of the Bardo desktop portal backend: the stream apps read a shared screen
//! from, the loop that moves frames from a capture into it, and the pixel conversion between.
//!
//! - [`FrameSink`] is the seam in front of `PipeWire`; [`PipewireSink`] is the real one and
//!   [`FakeSink`] records what it was given.
//! - [`Pump`] is the loop body between a [`portal_wayland::CaptureSource`] and a sink.
//! - [`convert`] changes a frame's pixel layout into the one the consumer picked.

mod pixels;
mod sink;
mod stream;

pub use pixels::{Layout, convert};
pub use sink::PipewireSink;
pub use stream::{FakeSink, FrameSink, Pump, PumpOutcome, QUIET_TURN, SinkError};
