//! The journal contract (design/31 §6.1): what the engine keeps of one dataset of one account,
//! and the way it keeps it.
//!
//! Every change is an [`Op`] list applied in one transaction (`apply`), so the engine's steps are
//! the unit of a crash: either a step's rows are all there or none are. The rows are sync-core's
//! values (`sync_core::journal`); a [`JournalStore`] only stores them. The engine is sans-IO
//! because it only sees this trait; `sync-journal-sqlite` is the store the daemon runs.

use sync_core::{
    ItemPath, JournalItem, LocalId, RemoteId, StoredAnchor, StoredConflict, StoredTombstone,
};

/// Why the journal could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    /// The store refused, or its file could not be made or opened; it says why in its own words.
    #[error("{0}")]
    Store(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// A row holds something this syncd cannot read.
    #[error("journal: {0}")]
    Corrupt(String),
}

/// One change to the journal. A list of them commits together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Inserts the item, or replaces the row with its local id.
    PutItem(JournalItem),
    /// Removes the item.
    DeleteItem(LocalId),
    /// Stores where the feed stands.
    SetAnchor(StoredAnchor),
    /// Forgets the anchor (it expired).
    ClearAnchor,
    /// Stores a tombstone, or replaces the one for that id.
    PutTombstone(StoredTombstone),
    /// Marks the tombstone of that id acknowledged.
    Acknowledge(RemoteId),
    /// Drops every acknowledged tombstone.
    CompactTombstones,
    /// Stores a conflict.
    AddConflict(StoredConflict),
    /// Removes a stored conflict by number.
    DropConflict(i64),
}

/// The journal of one dataset of one account.
pub trait JournalStore: std::fmt::Debug + Send + Sync {
    /// How many transactions this handle has committed (a test counts the cut points).
    fn writes(&self) -> usize;

    /// Commits `ops` together.
    ///
    /// # Errors
    ///
    /// Returns a [`JournalError`] when the transaction cannot be written; then none of `ops` is
    /// kept.
    fn apply(&self, ops: &[Op]) -> Result<(), JournalError>;

    /// Every item, by local id.
    ///
    /// # Errors
    ///
    /// Returns a [`JournalError`] when the store cannot be read.
    fn items(&self) -> Result<Vec<JournalItem>, JournalError>;

    /// The item the replica knows as `id`.
    ///
    /// # Errors
    ///
    /// Returns a [`JournalError`] when the store cannot be read.
    fn item_by_remote(&self, id: &RemoteId) -> Result<Option<JournalItem>, JournalError>;

    /// The item waiting to be uploaded for the first time at `path` (no remote id yet).
    ///
    /// # Errors
    ///
    /// Returns a [`JournalError`] when the store cannot be read.
    fn new_item_at(&self, path: &ItemPath) -> Result<Option<JournalItem>, JournalError>;

    /// The tombstone kept for `id`, if any.
    ///
    /// # Errors
    ///
    /// Returns a [`JournalError`] when the store cannot be read.
    fn tombstone_of(&self, id: &RemoteId) -> Result<Option<StoredTombstone>, JournalError> {
        Ok(self
            .tombstones()?
            .into_iter()
            .find(|stored| stored.tombstone.id == *id))
    }

    /// The anchor, if one is stored.
    ///
    /// # Errors
    ///
    /// Returns a [`JournalError`] when the store cannot be read.
    fn anchor(&self) -> Result<Option<StoredAnchor>, JournalError>;

    /// Every tombstone, by remote id.
    ///
    /// # Errors
    ///
    /// Returns a [`JournalError`] when the store cannot be read.
    fn tombstones(&self) -> Result<Vec<StoredTombstone>, JournalError>;

    /// Every stored conflict, oldest first.
    ///
    /// # Errors
    ///
    /// Returns a [`JournalError`] when the store cannot be read.
    fn conflicts(&self) -> Result<Vec<StoredConflict>, JournalError>;
}
