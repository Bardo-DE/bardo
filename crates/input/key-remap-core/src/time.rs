//! Monotonic instants: the shared [`Mono`] of `base-time`.
//!
//! The pure core never reads a clock: every step is handed `now`.

pub use base_time::Mono;
