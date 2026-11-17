//! Settling a stored conflict.
//!
//! The typed words of a resolution, and the request that goes from the hub to the dataset's driver (which owns the engine and settles it between
//! cycles, so nothing races a cycle writing the journal).

use sync_core::Resolution;
use tokio::sync::oneshot;

/// A stored conflict's number: the one `Sync1.Conflict` carries under `number`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConflictNumber(pub i64);

/// What `how` said (`keep_local`, `keep_remote`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct How(pub Resolution);

/// Why a conflict was not settled.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettleError {
    /// There is no such dataset for the caller (never registered, gone, or not the caller's to
    /// see): one answer for all, so a name is never confirmed to a caller who may not have it.
    #[error("no such dataset")]
    NoSuchDataset,
    /// The caller sees the dataset but is not its owning app.
    #[error("the dataset belongs to another app")]
    NotOwner,
    /// The conflict is unknown or already settled.
    #[error("no such conflict, or it is already settled")]
    NoSuchConflict,
    /// The journal or the local side failed.
    #[error("the conflict could not be settled: {0}")]
    Failed(String),
}

/// One request to a dataset's driver.
#[derive(Debug)]
pub struct Settle {
    /// The conflict.
    pub number: ConflictNumber,
    /// The side that wins.
    pub how: How,
    /// Where the driver answers.
    pub reply: oneshot::Sender<Result<(), SettleError>>,
}

/// Why a discard was not let through.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfirmError {
    /// There is no such dataset for the caller (as [`SettleError::NoSuchDataset`]).
    #[error("no such dataset")]
    NoSuchDataset,
    /// Nothing is held for confirmation: it never was, or the discard was already confirmed.
    #[error("nothing is held for confirmation")]
    NothingHeld,
}

/// One request to a dataset's driver: let the held discard through.
#[derive(Debug)]
pub struct Confirm {
    /// Where the driver answers.
    pub reply: oneshot::Sender<Result<(), ConfirmError>>,
}

/// Why "Sync now" did not start a cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SyncNowError {
    /// There is no such dataset (never registered, or gone).
    #[error("no such dataset")]
    NoSuchDataset,
    /// The person paused the dataset: it does not cycle until it is resumed.
    #[error("the dataset is paused")]
    Paused,
}
