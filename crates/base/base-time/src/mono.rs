//! The monotonic reading, for what is paced rather than dated (a rate limit's window).

use std::ops::{Add, Sub};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// Time since an arbitrary start, on a clock that never goes back. Two readings of the same clock
/// compare and subtract; readings of different clocks mean nothing together.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mono(pub Duration);

impl Mono {
    /// The reading `ms` milliseconds after the clock's origin; for fixtures.
    #[must_use]
    pub const fn from_millis(ms: u64) -> Self {
        Mono(Duration::from_millis(ms))
    }

    /// How long after `earlier` this is; zero if `earlier` is later.
    #[must_use]
    pub fn since(self, earlier: Mono) -> Duration {
        self.0.saturating_sub(earlier.0)
    }
}

impl Add<Duration> for Mono {
    type Output = Mono;

    fn add(self, rhs: Duration) -> Mono {
        Mono(self.0.saturating_add(rhs))
    }
}

impl Sub for Mono {
    type Output = Duration;

    fn sub(self, rhs: Mono) -> Duration {
        self.since(rhs)
    }
}

/// A monotonic reading. It is a seam beside [`crate::Clock`] so that a test moves it by hand.
pub trait Monotonic: Send + Sync {
    /// Now, on a clock that never goes back.
    fn now(&self) -> Mono;
}

impl std::fmt::Debug for dyn Monotonic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Monotonic").field(&self.now()).finish()
    }
}

/// A monotonic clock that stands still, at zero, until a test moves it with
/// [`ManualMonotonic::advance`].
#[derive(Debug, Default)]
pub struct ManualMonotonic {
    moved: Mutex<Duration>,
}

impl ManualMonotonic {
    /// A clock standing at zero.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Moves it on by `by`.
    pub fn advance(&self, by: Duration) {
        *self.moved.lock().unwrap_or_else(PoisonError::into_inner) += by;
    }
}

impl Monotonic for ManualMonotonic {
    fn now(&self) -> Mono {
        Mono(*self.moved.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

impl<T: Monotonic + ?Sized> Monotonic for Arc<T> {
    fn now(&self) -> Mono {
        (**self).now()
    }
}

impl<T: Monotonic + ?Sized> Monotonic for &T {
    fn now(&self) -> Mono {
        (**self).now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manual_clock_stands_still_until_moved() {
        let clock = ManualMonotonic::new();
        let start = clock.now();
        assert_eq!(clock.now(), start);
        clock.advance(Duration::from_secs(2));
        assert_eq!(clock.now() - start, Duration::from_secs(2));
        assert_eq!(start.since(clock.now()), Duration::ZERO);
    }
}
