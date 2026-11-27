//! A virtual clock: it says the same instant until a test moves it, and the router's timers
//! (`Sleeper::sleep`) complete only when the test has advanced it far enough. No wall time passes.

use action_router::{Flag, Queue};
use action_vocab::Seconds;
use base_time::{ManualClock, Sleeper, UnixSeconds};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Default)]
struct Sky {
    /// Virtual time since the clock was made.
    elapsed: Duration,
    /// The timers waiting: when each is due, and its flag.
    timers: Vec<(Duration, Arc<Flag>)>,
    /// Every wait the router asked for, in order.
    asked: Vec<Duration>,
}

/// A clock that says the same instant until `advance` moves it. `sleep(Duration::ZERO)` completes at
/// once; any other wait completes when the test has advanced the clock by that much.
#[derive(Debug, Clone)]
pub struct FixedClock {
    wall: Arc<ManualClock>,
    sky: Arc<Mutex<Sky>>,
    arrivals: Arc<Queue<Duration>>,
}

impl FixedClock {
    /// A clock at `start`.
    #[must_use]
    pub fn at(start: UnixSeconds) -> Self {
        Self {
            wall: Arc::new(ManualClock::new(start)),
            sky: Arc::default(),
            arrivals: Queue::new(),
        }
    }

    fn sky(&self) -> std::sync::MutexGuard<'_, Sky> {
        self.sky
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Moves virtual time forward and completes every timer that is now due.
    pub fn advance(&self, by: Duration) {
        let due: Vec<Arc<Flag>> = {
            let mut sky = self.sky();
            sky.elapsed += by;
            let now = sky.elapsed;
            let (due, waiting) = std::mem::take(&mut sky.timers)
                .into_iter()
                .partition::<Vec<_>, _>(|(at, _)| *at <= now);
            sky.timers = waiting;
            due.into_iter().map(|(_, flag)| flag).collect()
        };
        for flag in &due {
            flag.raise();
        }
    }

    /// Moves the instant `now` reports forward by `by`. Timers are not touched (`advance`
    /// moves those): this is for a rule that reads how old something is.
    pub fn pass(&self, by: Seconds) {
        self.wall.advance(u64::from(by.0));
    }

    /// Every wait the router has asked for so far, in order.
    #[must_use]
    pub fn asked(&self) -> Vec<Duration> {
        self.sky().asked.clone()
    }

    /// Resolves with the next wait the router asks for that has not been read yet: a test waits
    /// on this instead of on time.
    pub async fn next_ask(&self) -> Duration {
        self.arrivals.next().await
    }
}

impl base_time::Clock for FixedClock {
    fn now(&self) -> UnixSeconds {
        base_time::Clock::now(&*self.wall)
    }
}

impl Sleeper for FixedClock {
    async fn sleep(&self, how_long: Duration) {
        let flag = {
            let mut sky = self.sky();
            sky.asked.push(how_long);
            if how_long.is_zero() {
                None
            } else {
                let flag = Flag::new();
                let due = sky.elapsed + how_long;
                sky.timers.push((due, Arc::clone(&flag)));
                Some(flag)
            }
        };
        self.arrivals.push(how_long);
        if let Some(flag) = flag {
            flag.up().await;
        }
    }
}
