//! The system clock: read here, in the daemon, and nowhere below it.

use base_clock::{SystemClock as System, SystemMonotonic, TokioSleeper};
use base_time::{Mono, Monotonic, Sleeper, UnixSeconds};
use computer_use_desktop::Stamp;
use computer_use_run::Clock;
use std::time::Duration;

/// The system's clock, with its origin at construction. Copies share the origin: capture
/// stamps its frames on the same clock the run machine reads.
#[derive(Debug, Clone, Copy)]
pub struct SystemClock {
    origin: Mono,
}

impl SystemClock {
    /// A clock whose `Stamp`s count from now.
    #[must_use]
    pub fn start() -> Self {
        Self {
            origin: SystemMonotonic.now(),
        }
    }
}

impl base_time::Clock for SystemClock {
    fn now(&self) -> UnixSeconds {
        base_time::Clock::now(&System)
    }
}

impl Sleeper for SystemClock {
    fn sleep(&self, how_long: Duration) -> impl Future<Output = ()> + Send {
        TokioSleeper.sleep(how_long)
    }
}

/// The reading counts from the clock's origin, so capture, which takes a `Monotonic`, stamps
/// its frames on the clock the run machine reads.
impl Monotonic for SystemClock {
    fn now(&self) -> Mono {
        Mono(SystemMonotonic.now().since(self.origin))
    }
}

impl Clock for SystemClock {
    fn stamp(&self) -> Stamp {
        Stamp(u64::try_from(Monotonic::now(self).0.as_millis()).unwrap_or(u64::MAX))
    }
}
