//! The one bound on a call to a peer.

use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::task::Poll;
use std::time::Duration;

/// The call did not finish within its bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("no answer within {after:?}")]
pub struct TimedOut {
    /// The bound that passed.
    pub after: Duration,
}

/// Runs `work` for at most `limit`; the call is dropped, and so cancelled at its next await, when
/// the time passes. The timer is runtime-free, so this works under any executor.
///
/// # Errors
///
/// [`TimedOut`] when `work` does not finish within `limit`.
pub async fn within<T>(limit: Duration, work: impl Future<Output = T>) -> Result<T, TimedOut> {
    let mut work = pin!(work);
    let mut timer = futures_timer::Delay::new(limit);
    poll_fn(move |cx| {
        if let Poll::Ready(done) = work.as_mut().poll(cx) {
            return Poll::Ready(Ok(done));
        }
        match Pin::new(&mut timer).poll(cx) {
            Poll::Ready(()) => Poll::Ready(Err(TimedOut { after: limit })),
            Poll::Pending => Poll::Pending,
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::pending;

    const SHORT: Duration = Duration::from_millis(30);

    #[tokio::test]
    async fn a_call_that_answers_in_time_gives_its_answer() {
        assert_eq!(within(SHORT, async { 7 }).await, Ok(7));
    }

    #[tokio::test]
    async fn a_call_that_never_answers_times_out() {
        let got = within(SHORT, pending::<()>()).await;
        assert_eq!(got, Err(TimedOut { after: SHORT }));
    }
}
