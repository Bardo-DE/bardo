//! How a replica refuses: conflicts are values the journal stores, never hidden overwrites.

use crate::item::{BaseVersion, RemoteId, RemoteVersion};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// The error that made a replica's answer unusable, kept so a log can walk the chain.
pub type Cause = Arc<dyn std::error::Error + Send + Sync>;

/// A write whose base is not the replica's current version.
///
/// The journal keeps it as an object `{item, base, local, remote}` (the local side is the journal's
/// own) and the owning app resolves it by its dataset's rule.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Conflict {
    /// The item.
    pub item: RemoteId,
    /// What the write was based on.
    pub base: BaseVersion,
    /// What the replica has.
    pub remote: RemoteSide,
}

/// The replica's side of a conflict.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum RemoteSide {
    /// Changed to this version.
    Changed(RemoteVersion),
    /// Deleted, at this version.
    Deleted(RemoteVersion),
    /// Created by someone else while the writer believed it absent.
    Exists(RemoteId, RemoteVersion),
}

/// Seconds to wait before retrying (`Retry-After`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RetryAfter(pub u32);

/// Why a write was refused.
///
/// Two refusals are equal when they are the same kind of refusal with the same values; two
/// [`PutError::Failed`] are equal when they ask for the same wait, whatever caused them.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PutError {
    /// The base is stale.
    #[error("conflict on {:?}", .0.item)]
    Conflict(Conflict),
    /// The store is full.
    #[error("quota exceeded")]
    Quota,
    /// The account may not write there.
    #[error("forbidden")]
    Forbidden,
    /// Try again later.
    #[error("transient; retry after {} s", .0.0)]
    Transient(RetryAfter),
    /// The account's sign-in was refused: nothing more is sent until it is signed in again.
    #[error("unauthorized")]
    Unauthorized,
    /// The item is in use on the replica (locked there): try again after the wait.
    #[error("in use; retry after {} s", .0.0)]
    InUse(RetryAfter),
    /// The replica does not take the item's name or path.
    #[error("name refused")]
    NameRefused,
    /// The replica refused the change for a reason of its own, which no name taken explains.
    #[error("declined")]
    Declined,
    /// The write could not be made or its answer used (no connection, a body that does not parse,
    /// content that could not be read): try again after the wait. It keeps the cause.
    #[error("failed; retry after {} s", .0.0)]
    Failed(RetryAfter, #[source] Cause),
}

impl PutError {
    /// A write that failed because of `source`: try again after `after`.
    pub fn failed(
        after: RetryAfter,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Failed(after, Arc::new(source))
    }
}

impl PartialEq for PutError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Conflict(a), Self::Conflict(b)) => a == b,
            (Self::Transient(a), Self::Transient(b))
            | (Self::InUse(a), Self::InUse(b))
            | (Self::Failed(a, _), Self::Failed(b, _)) => a == b,
            (Self::Quota, Self::Quota)
            | (Self::Forbidden, Self::Forbidden)
            | (Self::Unauthorized, Self::Unauthorized)
            | (Self::NameRefused, Self::NameRefused)
            | (Self::Declined, Self::Declined) => true,
            _ => false,
        }
    }
}

impl Eq for PutError {}

/// Why an online document could not be exported as a file.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExportError {
    /// The backend exports no file of it (a Form, a Site, a Map, or a backend with no export).
    #[error("no file format")]
    Unsupported,
    /// The document is larger than the backend exports.
    #[error("too large to export")]
    TooLarge,
    /// The export could not be read.
    #[error(transparent)]
    Read(#[from] ReplicaError),
}

/// Why a read failed.
///
/// Two failures are equal when they are the same kind of failure with the same values; two
/// [`ReplicaError::Failed`] are equal when they ask for the same wait, whatever caused them.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ReplicaError {
    /// The anchor is no longer valid (Dropbox 409, Graph resync, an expired page token):
    /// list from `Cursor::Start` and reconcile by content hash, uploading nothing known.
    #[error("anchor expired")]
    AnchorExpired,
    /// The account's token was refused.
    #[error("unauthorized")]
    Unauthorized,
    /// Try again later.
    #[error("transient; retry after {} s", .0.0)]
    Transient(RetryAfter),
    /// The item or the dataset folder is gone.
    #[error("gone")]
    Gone,
    /// The account has not given the access the replica needs yet (the whole of a drive, asked
    /// when first used): nothing is retried until the person gives it.
    #[error("access not given yet")]
    NeedsConsent,
    /// The read could not be made or its answer used (no connection, a body that does not
    /// parse): try again after the wait. It keeps the cause.
    #[error("failed; retry after {} s", .0.0)]
    Failed(RetryAfter, #[source] Cause),
}

impl ReplicaError {
    /// A read that failed because of `source`: try again after `after`.
    pub fn failed(
        after: RetryAfter,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Failed(after, Arc::new(source))
    }
}

impl PartialEq for ReplicaError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Transient(a), Self::Transient(b)) | (Self::Failed(a, _), Self::Failed(b, _)) => {
                a == b
            }
            (Self::AnchorExpired, Self::AnchorExpired)
            | (Self::Unauthorized, Self::Unauthorized)
            | (Self::Gone, Self::Gone)
            | (Self::NeedsConsent, Self::NeedsConsent) => true,
            _ => false,
        }
    }
}

impl Eq for ReplicaError {}
