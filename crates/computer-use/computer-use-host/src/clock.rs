//! The system clock: read here, in the daemon, and nowhere below it.

use base_time::UnixSeconds;
use computer_use_desktop::Stamp;
use computer_use_run::Clock;
use std::time::Instant;

/// The system's clock, with its origin at construction. Copies share the origin: capture
/// stamps its frames on the same clock the run machine reads.
#[derive(Debug, Clone, Copy)]
pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    /// A clock whose `Stamp`s count from now.
    #[must_use]
    pub fn start() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl base_time::Clock for SystemClock {
    fn now(&self) -> UnixSeconds {
        base_time::Clock::now(&base_time::SystemClock)
    }
}

impl Clock for SystemClock {
    fn stamp(&self) -> Stamp {
        Stamp(u64::try_from(self.origin.elapsed().as_millis()).unwrap_or(u64::MAX))
    }

    fn after(&self, limit: std::time::Duration) -> impl std::future::Future<Output = ()> + Send {
        tokio::time::sleep(limit)
    }
}
