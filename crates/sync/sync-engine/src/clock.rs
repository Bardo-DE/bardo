//! What time it is, as a seam: the engine dates tombstones, anchors and conflicts by it.
//!
//! `ManualClock` is here because a test shares it through an `Arc` and sets it with `&self`,
//! which `base_time::FixedClock` (set with `&mut self`) cannot do. The clock that runs
//! faster than real time for the daemon's tests is `sync_driver::clock::Wall`.

use base_time::Clock;
use std::sync::atomic::{AtomicU64, Ordering};

/// A clock a test sets.
#[derive(Debug, Default)]
pub struct ManualClock(std::sync::atomic::AtomicI64);

impl ManualClock {
    /// A clock at `at`.
    #[must_use]
    pub fn at(at: i64) -> Self {
        Self(std::sync::atomic::AtomicI64::new(at))
    }

    /// Moves it to `at`.
    pub fn set(&self, at: i64) {
        self.0.store(at, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> base_time::UnixSeconds {
        base_time::UnixSeconds(self.0.load(std::sync::atomic::Ordering::Relaxed))
    }
}

/// The wall clock, in milliseconds, as a seam.
pub trait Millis: std::fmt::Debug + Send + Sync {
    /// Now.
    fn now_ms(&self) -> u64;
}

/// A millisecond clock a test sets.
#[derive(Debug, Default)]
pub struct ManualMillis(AtomicU64);

impl ManualMillis {
    /// A clock at `ms`.
    #[must_use]
    pub fn at(ms: u64) -> Self {
        Self(AtomicU64::new(ms))
    }

    /// Moves it to `ms` (it may go backwards: a device's clock can).
    pub fn set(&self, ms: u64) {
        self.0.store(ms, Ordering::Relaxed);
    }
}

impl Millis for ManualMillis {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}
