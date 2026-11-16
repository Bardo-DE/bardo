//! The system clock: wall-clock seconds, and deadlines that need no async runtime. A deadline
//! sleeps on a short-lived thread of its own (see [`crate::thread_sleep`]) that is gone when the
//! deadline future is dropped, so it works under any executor.

use crate::thread_sleep;
use action_vocab::{Clock, Millis};
use base_time::UnixSeconds;
use std::time::Duration;

/// The one system `Clock`: `base_time::SystemClock`'s seconds, and deadlines on a thread.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl base_time::Clock for SystemClock {
    fn now(&self) -> UnixSeconds {
        base_time::Clock::now(&base_time::SystemClock)
    }
}

impl Clock for SystemClock {
    async fn after(&self, wait: Millis) {
        if wait.0 > 0 {
            thread_sleep(Duration::from_millis(u64::from(wait.0))).await;
        }
    }
}
