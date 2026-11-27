//! The sleep seam: what waits for a duration. The real one is in `base-clock`; a test uses
//! [`ManualSleeper`], which waits for no time at all.

use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// Waits for a duration.
pub trait Sleeper: Send + Sync {
    /// Returns after `how_long`.
    fn sleep(&self, how_long: Duration) -> impl Future<Output = ()> + Send;
}

impl<T: Sleeper + ?Sized> Sleeper for Arc<T> {
    fn sleep(&self, how_long: Duration) -> impl Future<Output = ()> + Send {
        (**self).sleep(how_long)
    }
}

impl<T: Sleeper + ?Sized> Sleeper for &T {
    fn sleep(&self, how_long: Duration) -> impl Future<Output = ()> + Send {
        (**self).sleep(how_long)
    }
}

trait DynSleeper: Send + Sync {
    fn sleep_boxed(&self, how_long: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

impl<S: Sleeper> DynSleeper for S {
    fn sleep_boxed(&self, how_long: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(self.sleep(how_long))
    }
}

/// Any [`Sleeper`], shared and cloneable, so a type holds one without a type parameter.
#[derive(Clone)]
pub struct SharedSleeper(Arc<dyn DynSleeper>);

impl SharedSleeper {
    /// Shares `sleeper`.
    pub fn new(sleeper: impl Sleeper + 'static) -> Self {
        Self(Arc::new(sleeper))
    }
}

impl std::fmt::Debug for SharedSleeper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SharedSleeper")
    }
}

impl Sleeper for SharedSleeper {
    fn sleep(&self, how_long: Duration) -> impl Future<Output = ()> + Send {
        self.0.sleep_boxed(how_long)
    }
}

/// A sleeper for tests: every sleep returns at once and is written down.
#[derive(Debug, Default)]
pub struct ManualSleeper {
    slept: Mutex<Vec<Duration>>,
}

impl ManualSleeper {
    /// A sleeper that has slept for nothing yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The durations asked for so far, in order.
    #[must_use]
    pub fn slept(&self) -> Vec<Duration> {
        self.slept
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Sleeper for ManualSleeper {
    fn sleep(&self, how_long: Duration) -> impl Future<Output = ()> + Send {
        self.slept
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(how_long);
        std::future::ready(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_manual_sleeper_returns_at_once_and_remembers() {
        let sleeper = ManualSleeper::new();
        sleeper.sleep(Duration::from_secs(60)).await;
        sleeper.sleep(Duration::from_secs(1)).await;
        assert_eq!(
            sleeper.slept(),
            vec![Duration::from_secs(60), Duration::from_secs(1)]
        );
    }
}
