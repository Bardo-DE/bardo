//! The clock the daemon's engines run on: the system clock, optionally running faster than real
//! time for a test. (`sync_engine::clock::ManualClock` is the clock a test sets by hand.)

use base_time::{Clock, SystemClock};

/// The system clock, running `scale` times faster than real time (see `scheduler::Settings::time_scale`); with a scale of 1 it is exactly [`SystemClock`].
///
/// Every `Wall` in a process counts from the same start, so the engines of one test agree on the
/// time.
#[derive(Debug, Clone, Copy)]
pub struct Wall {
    scale: u32,
}

impl Wall {
    /// The clock of a scheduler that runs `scale` seconds to the real second.
    #[must_use]
    pub fn scaled(scale: u32) -> Self {
        Self {
            scale: scale.max(1),
        }
    }
}

impl Clock for Wall {
    fn now(&self) -> base_time::UnixSeconds {
        static START: std::sync::OnceLock<(std::time::Instant, base_time::UnixSeconds)> =
            std::sync::OnceLock::new();

        if self.scale == 1 {
            return SystemClock.now();
        }
        let (began, at) = START.get_or_init(|| (std::time::Instant::now(), SystemClock.now()));
        let passed = began.elapsed().as_millis() * u128::from(self.scale) / 1000;
        base_time::UnixSeconds(at.0.saturating_add(i64::try_from(passed).unwrap_or(i64::MAX)))
    }
}
