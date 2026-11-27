//! The system's time: the real [`SystemClock`] (wall clock, and a timer to wait on),
//! [`SystemMonotonic`] (monotonic) and [`TokioSleeper`] (the timer of a tokio program), the
//! implementations of the seams in `base-time`. A program reads
//! the time here, at its edge, and passes the seam down; tests use the doubles in `base-time`.

use base_time::{Clock, Mono, Monotonic, Sleeper, UnixSeconds};
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The system's wall clock, and its timer. Waiting needs no async runtime: a sleep waits on a
/// timer thread, so it works under any executor.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> UnixSeconds {
        let since = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        UnixSeconds(i64::try_from(since.as_secs()).unwrap_or(i64::MAX))
    }

    fn now_ms(&self) -> u64 {
        let since = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
    }
}

/// The system's monotonic clock. Its readings count from the first one taken in the process, so
/// every copy agrees.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemMonotonic;

impl Monotonic for SystemMonotonic {
    fn now(&self) -> Mono {
        static START: OnceLock<Instant> = OnceLock::new();
        Mono(START.get_or_init(Instant::now).elapsed())
    }
}

impl Sleeper for SystemClock {
    async fn sleep(&self, how_long: Duration) {
        if !how_long.is_zero() {
            futures_timer::Delay::new(how_long).await;
        }
    }
}

#[cfg(feature = "tokio")]
/// A sleeper on tokio's timer, for a program that runs on tokio: it follows the runtime's clock,
/// so a test with paused time does not wait. It must be awaited inside a runtime.
#[derive(Debug, Clone, Copy, Default)]
pub struct TokioSleeper;

#[cfg(feature = "tokio")]
impl Sleeper for TokioSleeper {
    fn sleep(&self, how_long: Duration) -> impl Future<Output = ()> + Send {
        tokio::time::sleep(how_long)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_monotonic_clock_never_goes_back() {
        let first = SystemMonotonic.now();
        assert!(SystemMonotonic.now() >= first);
    }

    #[test]
    fn the_wall_clock_is_after_the_year_2020() {
        assert!(SystemClock.now().0 > 1_577_836_800);
    }

    #[test]
    fn a_sleep_of_no_time_is_done_at_once() {
        let mut sleeping = std::pin::pin!(SystemClock.sleep(Duration::ZERO));
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        assert_eq!(sleeping.as_mut().poll(&mut cx), std::task::Poll::Ready(()));
    }

    #[cfg(feature = "tokio")]
    #[tokio::test(start_paused = true)]
    async fn a_tokio_sleep_follows_the_paused_runtime_clock() {
        TokioSleeper.sleep(Duration::from_secs(3600)).await;
    }

    #[tokio::test]
    async fn a_sleep_ends() {
        SystemClock.sleep(Duration::from_millis(5)).await;
    }
}
