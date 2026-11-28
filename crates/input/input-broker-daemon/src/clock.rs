//! Time as an argument: the service reads the clock in one place.

use base_time::{Mono, Monotonic};

/// Time since a [`Monotonic`] clock was first read: the counter the core's rate limiter
/// and idle timer compare.
#[derive(Debug, Clone)]
pub struct Uptime<M> {
    clock: M,
    start: Mono,
}

impl<M: Monotonic> Uptime<M> {
    /// A counter that reads zero now.
    pub fn new(clock: M) -> Self {
        let start = clock.now();
        Self { clock, start }
    }

    /// Time since the counter was made; never goes back.
    pub fn now(&self) -> Mono {
        Mono(self.clock.now().since(self.start))
    }
}
