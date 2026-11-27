//! The wall clock as [`Clock`], with the test clocks that stand in for the system's (which is in
//! `base-clock`).

use crate::UnixSeconds;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// The time now. Grants, audit entries, tombstones, sessions and anchors are dated through it;
/// tests pass a fixed one.
pub trait Clock: Send + Sync {
    /// Now.
    fn now(&self) -> UnixSeconds;

    /// Now in milliseconds since the epoch, for what orders events within a second (a hybrid
    /// logical clock). A clock that only knows whole seconds gives them times a thousand.
    fn now_ms(&self) -> u64 {
        u64::try_from(self.now().0)
            .unwrap_or(0)
            .saturating_mul(1000)
    }
}

impl std::fmt::Debug for dyn Clock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Clock").field(&self.now()).finish()
    }
}

/// A clock that always says the same time. A test sets it again with [`FixedClock::set`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedClock(pub UnixSeconds);

impl FixedClock {
    /// A clock at `at`.
    #[must_use]
    pub const fn new(at: UnixSeconds) -> Self {
        Self(at)
    }

    /// Moves it to `at`.
    pub fn set(&mut self, at: UnixSeconds) {
        self.0 = at;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> UnixSeconds {
        self.0
    }
}

/// A clock a test moves by hand, through `&self`.
///
/// One handle is shared between the code under test and the test (behind an `Arc`, or by
/// reference). It keeps milliseconds, so it also stands
/// in for a clock read to the millisecond; it may be set backwards, as a device's clock can.
#[derive(Debug, Default)]
pub struct ManualClock(AtomicU64);

impl ManualClock {
    /// A clock at `at`.
    #[must_use]
    pub fn new(at: UnixSeconds) -> Self {
        Self::at_ms(u64::try_from(at.0).unwrap_or(0).saturating_mul(1000))
    }

    /// A clock `ms` milliseconds after the epoch.
    #[must_use]
    pub const fn at_ms(ms: u64) -> Self {
        Self(AtomicU64::new(ms))
    }

    /// Moves it to `at`.
    pub fn set(&self, at: UnixSeconds) {
        self.set_ms(u64::try_from(at.0).unwrap_or(0).saturating_mul(1000));
    }

    /// Moves it to `ms` milliseconds after the epoch.
    pub fn set_ms(&self, ms: u64) {
        self.0.store(ms, Ordering::SeqCst);
    }

    /// Moves it on by `seconds`.
    pub fn advance(&self, seconds: u64) {
        self.0
            .fetch_add(seconds.saturating_mul(1000), Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> UnixSeconds {
        UnixSeconds(i64::try_from(self.now_ms() / 1000).unwrap_or(i64::MAX))
    }

    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

impl<T: Clock + ?Sized> Clock for Arc<T> {
    fn now(&self) -> UnixSeconds {
        (**self).now()
    }

    fn now_ms(&self) -> u64 {
        (**self).now_ms()
    }
}

impl<T: Clock + ?Sized> Clock for &T {
    fn now(&self) -> UnixSeconds {
        (**self).now()
    }

    fn now_ms(&self) -> u64 {
        (**self).now_ms()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manual_clock_moves_through_a_shared_reference() {
        let clock = ManualClock::new(UnixSeconds(10));
        clock.advance(5);
        assert_eq!(clock.now(), UnixSeconds(15));
        clock.set(UnixSeconds(3));
        assert_eq!(clock.now(), UnixSeconds(3));
        clock.set_ms(1_700_000_000_250);
        assert_eq!(clock.now(), UnixSeconds(1_700_000_000));
        assert_eq!(clock.now_ms(), 1_700_000_000_250);
        clock.set_ms(5);
        assert_eq!(clock.now_ms(), 5, "a clock may be set backwards");
    }
}
