//! Time as an argument: the service reads the clock in one place.

use base_time::Monotonic;
use input_broker_wire::Millis;
use std::time::Instant;

/// Milliseconds since a [`Monotonic`] clock was first read: the counter the wire crate's rate
/// limiter and idle timer compare.
#[derive(Debug, Clone)]
pub struct Uptime<M> {
    clock: M,
    start: Instant,
}

impl<M: Monotonic> Uptime<M> {
    /// A counter that reads zero now.
    pub fn new(clock: M) -> Self {
        let start = clock.now();
        Self { clock, start }
    }

    /// Milliseconds since the counter was made; never goes back.
    pub fn now(&self) -> Millis {
        let since = self.clock.now().saturating_duration_since(self.start);
        Millis(u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
    }
}
