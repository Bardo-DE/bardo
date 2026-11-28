//! The plain values of the seat guards, with no system call: what a connection claims, what a
//! failed `accept` means, how long a listener pauses and whether a refusal is logged. The pure
//! state machines of the socket daemons depend on these without linking the guards' I/O, which
//! stay in `seat-guard`.

use std::time::Duration;

/// How long the listener is left alone after `accept` failed for a reason that retrying at once
/// will not clear (out of file descriptors).
pub const ACCEPT_PAUSE: Duration = Duration::from_secs(1);
/// How often one subscriber's refusal is written to the log.
pub const COMPLAINT_INTERVAL: Duration = Duration::from_secs(60);

/// What a failed `accept` means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptFailure {
    /// No connection is waiting: the loop is done for this pass.
    NothingPending,
    /// Interrupted: ask again.
    Retry,
    /// The connection stays pending, so polling would return at once and spin: leave the
    /// listener alone for a while.
    Pause,
}

/// Whether a refusal is written to the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voice {
    /// Write the refusal to the log.
    Speak,
    /// Say nothing: this subscriber complained recently.
    Silent,
}

/// What a connection claims right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Claim {
    /// The shell of this user, and its process is still the one we held.
    Shell {
        /// The user whose shell it is.
        user: u32,
    },
    /// Anything else.
    Other,
}
