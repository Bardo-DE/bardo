//! The clock seam: this crate never reads the wall clock.

use memory_core::UnixSeconds;

/// Now, as seconds since the epoch. `memory_local::SystemClock` is the only reader of the real
/// clock; tests pass a stepped one.
pub trait Clock: Send + Sync {
    /// The current instant.
    fn now(&self) -> UnixSeconds;
}
