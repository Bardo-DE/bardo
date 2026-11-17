//! How long a call to another process may take, and the bound that enforces it.
//!
//! A peer can be slow, crashed, wedged behind a password prompt or simply absent, and a bus
//! call has no timeout of its own. Wrapping the call in [`within`] gives it one: the call either
//! answers inside its [`Deadline`] or ends as [`NotAnswering`], which a screen shows as "X is not
//! answering" instead of spinning forever.
//!
//! The deadlines are named, one per kind of wait ([`Deadline::READ`], [`Deadline::START`],
//! [`Deadline::ACTION`], [`Deadline::PROMPT`]); a caller picks one and never writes a bare
//! duration. This is the peer-facing layer over [`crate::timeout::within`], which takes a bare
//! duration. No runtime and no bus: a wall-clock delay and a future.

use crate::timeout::{TimedOut, within as bounded};
use std::time::Duration;

/// How long a call to a peer may take before it counts as not answering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Deadline(Duration);

impl Deadline {
    /// A read from a peer that is running: a property, a settings value, a name's owner. A
    /// healthy peer answers in milliseconds; this is slack for a busy machine.
    pub const READ: Deadline = Deadline(Duration::from_secs(10));
    /// A read the app waits on before it can draw (the first probe of what is installed, the
    /// first read of the desktop's appearance). Short, so a wedged peer costs a start a moment
    /// and no more; what arrives later is picked up by the follow.
    pub const START: Deadline = Deadline(Duration::from_secs(3));
    /// A call that changes something and returns when it is done: setting a value, creating a
    /// transaction, asking a service to start an action.
    pub const ACTION: Deadline = Deadline(Duration::from_secs(30));
    /// A wait on the person: a password prompt, a print dialog. Long, because they may be
    /// reading it, but not for ever; a peer that vanished is noticed sooner by watching its name.
    pub const PROMPT: Deadline = Deadline(Duration::from_secs(600));

    /// A deadline of `duration`, for a test that must not wait the real one out.
    #[must_use]
    pub const fn of(duration: Duration) -> Deadline {
        Deadline(duration)
    }

    /// How long it is.
    #[must_use]
    pub const fn duration(self) -> Duration {
        self.0
    }
}

/// A peer that did not answer inside its deadline. The words are for a person: "The package
/// manager is not answering."
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{peer} is not answering")]
pub struct NotAnswering {
    peer: &'static str,
    after: Duration,
}

impl NotAnswering {
    /// Who did not answer, as a screen names it ("The installer").
    #[must_use]
    pub fn peer(&self) -> &'static str {
        self.peer
    }

    /// How long it was given.
    #[must_use]
    pub fn after(&self) -> Duration {
        self.after
    }
}

/// Run `work` for at most `deadline`.
///
/// `peer` is the plain name a screen would use for whoever is being waited on ("The installer",
/// "Accounts"); it starts with a capital and names no protocol. The call is dropped, and so
/// cancelled at its next await, when the deadline passes.
///
/// # Errors
///
/// Returns `NotAnswering` when `work` does not finish before the deadline.
pub async fn within<T>(
    deadline: Deadline,
    peer: &'static str,
    work: impl Future<Output = T>,
) -> Result<T, NotAnswering> {
    bounded(deadline.duration(), work)
        .await
        .map_err(|TimedOut { after }| NotAnswering { peer, after })
}

#[cfg(test)]
mod tests {
    use super::{Deadline, NotAnswering, within};
    use std::future::pending;
    use std::time::Duration;

    const SHORT: Deadline = Deadline::of(Duration::from_millis(30));

    #[tokio::test]
    async fn a_call_that_answers_in_time_gives_its_answer() {
        assert_eq!(within(SHORT, "Accounts", async { 7 }).await, Ok(7));
    }

    #[tokio::test]
    async fn a_call_that_never_answers_ends_as_not_answering() {
        let got = within(SHORT, "Accounts", pending::<()>()).await;
        let Err(error) = got else {
            panic!("a pending call cannot answer");
        };
        assert_eq!(error.peer(), "Accounts");
        assert_eq!(error.after(), SHORT.duration());
        assert_eq!(error.to_string(), "Accounts is not answering");
    }

    #[tokio::test]
    async fn a_late_answer_is_not_waited_for() {
        let slow = async {
            tokio::time::sleep(Duration::from_secs(30)).await;
        };
        let started = std::time::Instant::now();
        let got: Result<(), NotAnswering> = within(SHORT, "Accounts", slow).await;
        assert!(got.is_err());
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
