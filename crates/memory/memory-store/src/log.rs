//! The log's seams: reading, writing and what can go wrong.

use crate::chain::Entry;
use crate::header::NewHeader;
use memory_core::{Checkpoint, Count, EventBody, Head, Seq, ThingRef, TimelineQuery};
use std::sync::Arc;

/// A page of the log, newest first: the timeline's own query.
pub type PageQuery = TimelineQuery;

/// Which role of a thing a lookup wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoleFilter {
    /// Subject or source.
    Either,
    /// Events about the thing.
    Subject,
    /// Events that came from the thing.
    Source,
}

/// Why a log operation failed. Two errors are equal when they are the same kind of failure and
/// name the same place or value; the cause of an `Unreadable` is not compared.
#[derive(Debug, Clone, thiserror::Error)]
pub enum LogError {
    /// The key does not open the database.
    #[error("the event log is locked")]
    Locked,
    /// The stored chain is not consistent at this entry.
    #[error("the event log is corrupt at {at:?}")]
    Corrupt {
        /// Where.
        at: Seq,
    },
    /// The disk is full.
    #[error("the event log is full")]
    Full,
    /// SQLite said no.
    #[error("sqlite: {0}")]
    Sqlite(String),
    /// A stored column does not read back (JSON that is not the value it should be, a kind that is
    /// not a kind tag).
    #[error("the event log holds an unreadable entry at {at:?}")]
    Unreadable {
        /// Where.
        at: Seq,
        /// What did not read.
        #[source]
        source: Arc<dyn std::error::Error + Send + Sync>,
    },
    /// A schema version this build does not know.
    #[error("unknown event log schema {found}")]
    Schema {
        /// What was found.
        found: u32,
    },
    /// The body does not match the digest in its header.
    #[error("body does not match its header's digest")]
    BadDigest,
    /// No entry at that sequence number.
    #[error("no entry at {0:?}")]
    NoSuchEntry(Seq),
}

impl LogError {
    /// A stored column of the entry at `at` that does not read, because of `source`.
    pub fn unreadable(at: Seq, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Unreadable {
            at,
            source: Arc::new(source),
        }
    }
}

impl PartialEq for LogError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Corrupt { at: a }, Self::Corrupt { at: b })
            | (Self::Unreadable { at: a, .. }, Self::Unreadable { at: b, .. })
            | (Self::NoSuchEntry(a), Self::NoSuchEntry(b)) => a == b,
            (Self::Sqlite(a), Self::Sqlite(b)) => a == b,
            (Self::Schema { found: a }, Self::Schema { found: b }) => a == b,
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }
}

impl Eq for LogError {}

/// Reading the log.
pub trait LogRead {
    /// The newest entry, or the checkpoint's position when the log is empty.
    ///
    /// # Errors
    ///
    /// Returns [`LogError`] when the log cannot be read.
    fn head(&self) -> Result<Head, LogError>;
    /// Where the retained entries start: genesis, or the last prune.
    ///
    /// # Errors
    ///
    /// Returns [`LogError`] when the log cannot be read.
    fn checkpoint(&self) -> Result<Checkpoint, LogError>;
    /// A page, newest first, from a cursor, filtered.
    ///
    /// # Errors
    ///
    /// Returns [`LogError`] when the log cannot be read.
    fn page(&self, q: &PageQuery) -> Result<Vec<Entry>, LogError>;
    /// The entries whose bodies name `thing` in the given role.
    ///
    /// # Errors
    ///
    /// Returns [`LogError`] when the log cannot be read.
    fn touching(&self, thing: &ThingRef, role: RoleFilter) -> Result<Vec<Seq>, LogError>;
    /// Every entry from `from`, ascending: for verification and index rebuilds.
    ///
    /// # Errors
    ///
    /// Returns [`LogError`] when the log cannot be read.
    fn scan(&self, from: Seq) -> Result<Vec<Entry>, LogError>;
}

/// Writing the log.
pub trait LogWrite: LogRead {
    /// Chains and stores a header and, unless the admission was header-only, its body.
    ///
    /// # Errors
    ///
    /// Returns [`LogError`] when the entry cannot be chained or stored.
    fn append(&mut self, header: NewHeader, body: Option<EventBody>) -> Result<Entry, LogError>;
    /// Erases bodies and their `things` rows; headers and digests stay.
    ///
    /// # Errors
    ///
    /// Returns [`LogError`] when the bodies cannot be erased.
    fn erase_bodies(&mut self, seqs: &[Seq]) -> Result<Count, LogError>;
    /// Removes the prefix up to and including `cut`, leaving a checkpoint. Prefix only.
    ///
    /// # Errors
    ///
    /// Returns [`LogError`] when the prefix cannot be removed or the checkpoint cannot be stored.
    fn prune_before(&mut self, cut: Seq) -> Result<Checkpoint, LogError>;
}
