//! The session lock's vocabulary (`ext-session-lock-v1`): which lock, how its surfaces are set up,
//! why a request failed, and where the lock is.

use serde::{Deserialize, Serialize};

/// One session lock, from `HostCtx::lock_session`. Never reused within a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LockId(pub u64);

/// How lock surfaces are set up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockConfig {
    /// The lock surfaces' name prefix: each is `<namespace>/<output name>` (the output's
    /// registry name when it has no connector name), for `HostCtx::surface_named` and stats.
    pub namespace: String,
}

impl Default for LockConfig {
    fn default() -> LockConfig {
        LockConfig {
            namespace: "lock".to_owned(),
        }
    }
}

/// Why `HostCtx::lock_session` could not ask for a lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum LockError {
    /// The compositor has no `ext_session_lock_manager_v1` (`KWin` 6.7, for one).
    #[error("the compositor does not provide ext_session_lock_manager_v1")]
    Unsupported,
    /// A lock of this host's is still in force (requested, locked or unlocking).
    #[error("session lock {0:?} is still in force")]
    InProgress(LockId),
}

/// Where a session lock is, as its documents see it (`LockHandle::phase`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockPhase {
    /// Asked for; the compositor has not answered yet.
    Requested,
    /// The compositor says the session is locked.
    Locked,
    /// An unlock was asked for; the documents are being told before the compositor is.
    Unlocking,
    /// `unlock_and_destroy` was sent (or the request was cancelled before `locked`).
    Unlocked,
    /// The compositor refused or ended the lock (`finished`).
    Finished,
}

impl LockPhase {
    /// Whether the lock is over: nothing moves it any more.
    #[must_use]
    pub fn ended(self) -> Ended {
        match self {
            LockPhase::Unlocked | LockPhase::Finished => Ended::Yes,
            LockPhase::Requested | LockPhase::Locked | LockPhase::Unlocking => Ended::No,
        }
    }
}

/// Whether a lock has ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ended {
    Yes,
    No,
}

/// What the host reports to the shell (`HostEvent::Lock`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockState {
    /// The compositor locked the session: only lock surfaces are shown and get input.
    Locked,
    /// The host unlocked the session (or cancelled the request before it was locked).
    Unlocked,
    /// The compositor refused the lock (another locker holds it) or ended it by its own means.
    Finished,
}
