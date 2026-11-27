//! Retry and backoff, done once, in a wrapper.
//!
//! Rig only classifies a failure and leaves the loop to the host. `ProviderError` says how it may
//! be retried (`RetryClass`, in `model-types`), `next_wait` is a pure function (the jitter is an input, so tests need no
//! random source), and [`Retrying`] runs the loop over a `Sleeper` (so tests need no clock).
//!
//! The rule that matters: never retry once the first event reached the sink. A half-delivered
//! turn is not idempotent for the planner.

use base_time::Sleeper;
use serde::{Deserialize, Serialize};
use std::ops::ControlFlow;
use std::time::Duration;

use model_types::{
    Attempt, Charges, ModelInfo, Permille, ProviderError, RetryClass, TurnEnd, TurnEvent,
    TurnRequest, TurnSink, UsageReport, WaitMs,
};

use crate::Provider;

/// How many tries, and the backoff between them (`base * 2^(n-1)`, capped at `cap`). The numbers
/// are settings the daemon reads; no default lives here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// How many tries in all, the first included.
    pub attempts: Attempt,
    /// The wait after the first try.
    pub base: WaitMs,
    /// The longest wait there is.
    pub cap: WaitMs,
}

/// How long to wait before try `attempt + 1`, or `None` when the class or the budget says stop.
///
/// `jitter` scales the wait down by up to that share, so callers spread out. No wait, a
/// server-requested one included, exceeds `policy.cap`.
#[must_use]
pub fn next_wait(
    policy: &RetryPolicy,
    attempt: Attempt,
    class: RetryClass,
    jitter: Permille,
) -> Option<WaitMs> {
    if class == RetryClass::Never || attempt.0 >= policy.attempts.0 {
        return None;
    }
    let doubled = u64::from(policy.base.0) << u32::from(attempt.0.saturating_sub(1)).min(31);
    let capped = doubled.min(u64::from(policy.cap.0));
    let jitter = u64::from(jitter.0.min(1000));
    let backoff = capped - capped * jitter / 1000;
    let wait = match class {
        // The server's number is capped like the backoff: one `Retry-After` must not park a turn.
        RetryClass::Overload(server) => backoff
            .max(u64::from(server.0) * 1000)
            .min(u64::from(policy.cap.0)),
        RetryClass::Never | RetryClass::Transient => backoff,
    };
    Some(WaitMs(u32::try_from(wait).unwrap_or(u32::MAX)))
}

/// Wraps a provider and retries a turn that failed before it delivered an event. `describe` is
/// retried the same way (it has no events).
#[derive(Debug, Clone)]
pub struct Retrying<P: Provider, S: Sleeper> {
    inner: P,
    policy: RetryPolicy,
    sleep: S,
    jitter: Permille,
}

impl<P: Provider, S: Sleeper> Retrying<P, S> {
    /// Retries `inner` by `policy`, waiting on `sleep` (a timer in a daemon, a recorder in a test).
    pub fn new(inner: P, policy: RetryPolicy, sleep: S) -> Self {
        Self {
            inner,
            policy,
            sleep,
            jitter: Permille(0),
        }
    }

    /// Shaves `jitter` (thousandths) off every backoff, so callers spread out. None by default.
    #[must_use]
    pub fn with_jitter(mut self, jitter: Permille) -> Self {
        self.jitter = jitter;
        self
    }
}

impl<P: Provider, S: Sleeper> Provider for Retrying<P, S> {
    async fn describe(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        let mut attempt = Attempt(1);
        loop {
            match self.inner.describe().await {
                Err(error) => self.pause(attempt, &error).await?,
                done => return done,
            }
            attempt = Attempt(attempt.0.saturating_add(1));
        }
    }

    async fn turn<K: TurnSink>(
        &self,
        request: &TurnRequest,
        sink: &mut K,
    ) -> Result<TurnEnd, ProviderError> {
        let mut counting = Counting {
            inner: sink,
            seen: 0,
        };
        let mut attempt = Attempt(1);
        loop {
            match self.inner.turn(request, &mut counting).await {
                Err(error) if counting.seen == 0 => self.pause(attempt, &error).await?,
                done => return done,
            }
            attempt = Attempt(attempt.0.saturating_add(1));
        }
    }

    fn charges(&self) -> Charges {
        self.inner.charges()
    }
}

impl<P: Provider, S: Sleeper> Retrying<P, S> {
    /// Sleeps the wait that follows try `attempt`, or hands `error` back when it may not be
    /// retried. The jitter comes from the sleeper, which owns the clock and any random source.
    async fn pause(&self, attempt: Attempt, error: &ProviderError) -> Result<(), ProviderError> {
        match next_wait(&self.policy, attempt, error.retry_class(), self.jitter) {
            Some(wait) => {
                self.sleep
                    .sleep(Duration::from_millis(u64::from(wait.0)))
                    .await;
                Ok(())
            }
            None => Err(error.clone()),
        }
    }
}

/// Counts the events that reach the caller's sink: a turn that has delivered one is never retried.
struct Counting<'a, K> {
    inner: &'a mut K,
    seen: usize,
}

impl<K: TurnSink> TurnSink for Counting<'_, K> {
    fn event(&mut self, event: TurnEvent) -> ControlFlow<()> {
        self.seen += 1;
        self.inner.event(event)
    }

    fn ended(&mut self, report: UsageReport) {
        self.inner.ended(report);
    }
}
