//! The one clock: what tells the time, and waits it out. The system's is in `action-store`, a
//! test's in `action-fake`; nothing else in the repo reads the time or sleeps for a deadline.

use crate::Millis;
use provenance::UnixSeconds;
use std::future::Future;

/// The one clock.
pub trait Clock: Send + Sync {
    /// Now.
    fn now(&self) -> UnixSeconds;
    /// Completes after `wait`: the deadline a host races a peer against (the real clock sleeps;
    /// a fake completes at once only for no time at all).
    fn after(&self, wait: Millis) -> impl Future<Output = ()> + Send;
}
