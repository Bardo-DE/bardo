//! The SQLite journal of one dataset of one account (design/31 §6.1): items, the anchor,
//! tombstones and conflicts, in one file under `$XDG_STATE_HOME/bardo/sync/<account>/`.
//!
//! Every change is an [`Op`] list applied in one transaction (`apply`), so the engine's steps
//! are the unit of a crash: either a step's rows are all there or none are. The rows are
//! sync-core's values (`sync_core::journal`); this file only stores them.

mod rows;
mod schema;

pub use schema::prepare_with;

use rusqlite::Connection;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use sync_core::{ItemPath, JournalItem, RemoteId, StoredAnchor, StoredConflict, StoredTombstone};
use sync_engine::journal::{JournalError, JournalStore, Op};

/// Why the SQLite journal could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum SqliteError {
    /// SQLite refused.
    #[error("journal: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// The file's directory could not be made.
    #[error("journal: {0}")]
    Io(#[from] std::io::Error),
    /// A row holds something this syncd cannot read.
    #[error("journal: {0}")]
    Corrupt(String),
    /// A test cut the process off before this write.
    #[cfg(test)]
    #[error("journal: cut before write")]
    Cut,
}

impl From<SqliteError> for JournalError {
    fn from(error: SqliteError) -> Self {
        match error {
            SqliteError::Corrupt(what) => JournalError::Corrupt(what),
            other => JournalError::Store(Box::new(other)),
        }
    }
}

/// A journal file, open.
#[derive(Debug)]
pub struct Journal {
    connection: Mutex<Connection>,
    writes: AtomicUsize,
    #[cfg(test)]
    cut: Mutex<Option<usize>>,
}

impl Journal {
    /// Opens the journal at `path`, creating it (and its directory); a file of another schema version is emptied.
    ///
    /// # Errors
    ///
    /// Returns a [`SqliteError`] when the directory or the database cannot be created, opened or
    /// set up.
    pub fn open(path: &Path) -> Result<Self, SqliteError> {
        Self::open_synchronous(path, "FULL")
    }

    /// A journal whose commits are not flushed to the disk, for tests.
    ///
    /// For the tests that make thousands of commits (the thousand photos), which prove the
    /// engine, not the disk. The crash tests keep `open`.
    ///
    /// # Errors
    ///
    /// Returns a [`SqliteError`] when the directory or the database cannot be created, opened or
    /// set up.
    #[cfg(any(test, feature = "testing"))]
    pub fn open_unflushed(path: &Path) -> Result<Self, SqliteError> {
        Self::open_synchronous(path, "OFF")
    }

    fn open_synchronous(path: &Path, synchronous: &str) -> Result<Self, SqliteError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut connection = Connection::open(path)?;
        connection.execute_batch(&format!("PRAGMA synchronous = {synchronous};"))?;
        schema::prepare(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
            writes: AtomicUsize::new(0),
            #[cfg(test)]
            cut: Mutex::new(None),
        })
    }

    fn connection(&self) -> MutexGuard<'_, Connection> {
        // A panic mid-statement leaves SQLite's own transaction to roll back.
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// How many transactions this handle has committed (a test counts the cut points).
    pub fn writes(&self) -> usize {
        self.writes.load(Ordering::Relaxed)
    }

    /// Makes the `n`th write from now (0 is the next) fail before it commits, and every one
    /// after it: the process died there.
    #[cfg(test)]
    pub(crate) fn cut_at(&self, n: usize) {
        *self.cut.lock().unwrap_or_else(PoisonError::into_inner) = Some(self.writes() + n);
    }

    /// Commits `ops` together.
    ///
    /// # Errors
    ///
    /// Returns a [`SqliteError`] when the transaction cannot be written (or a test cut the
    /// connection); then none of `ops` is kept.
    pub fn apply(&self, ops: &[Op]) -> Result<(), SqliteError> {
        #[cfg(test)]
        {
            let cut = *self.cut.lock().unwrap_or_else(PoisonError::into_inner);
            if cut.is_some_and(|at| self.writes() >= at) {
                return Err(SqliteError::Cut);
            }
        }
        let mut connection = self.connection();
        let tx = connection.transaction()?;
        for op in ops {
            rows::apply(&tx, op)?;
        }
        tx.commit()?;
        self.writes.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Every item, by local id.
    ///
    /// # Errors
    ///
    /// Returns a [`SqliteError`] when the database cannot be read.
    pub fn items(&self) -> Result<Vec<JournalItem>, SqliteError> {
        rows::items(&self.connection())
    }

    /// The item the replica knows as `id`.
    ///
    /// # Errors
    ///
    /// Returns a [`SqliteError`] when the database cannot be read.
    pub fn item_by_remote(&self, id: &RemoteId) -> Result<Option<JournalItem>, SqliteError> {
        rows::item_where(&self.connection(), "remote_id = ?1", &id.0)
    }

    /// The item waiting to be uploaded for the first time at `path` (no remote id yet).
    ///
    /// # Errors
    ///
    /// Returns a [`SqliteError`] when the database cannot be read.
    pub fn new_item_at(&self, path: &ItemPath) -> Result<Option<JournalItem>, SqliteError> {
        rows::item_where(
            &self.connection(),
            "remote_id IS NULL AND state = 'pending_upload' AND path = ?1",
            &path.0,
        )
    }

    /// The tombstone kept for `id`, if any.
    ///
    /// # Errors
    ///
    /// Returns a [`SqliteError`] when the database cannot be read.
    pub fn tombstone_of(&self, id: &RemoteId) -> Result<Option<StoredTombstone>, SqliteError> {
        Ok(self
            .tombstones()?
            .into_iter()
            .find(|stored| stored.tombstone.id == *id))
    }

    /// The anchor, if one is stored.
    ///
    /// # Errors
    ///
    /// Returns a [`SqliteError`] when the database cannot be read.
    pub fn anchor(&self) -> Result<Option<StoredAnchor>, SqliteError> {
        rows::anchor(&self.connection())
    }

    /// Every tombstone, by remote id.
    ///
    /// # Errors
    ///
    /// Returns a [`SqliteError`] when the database cannot be read.
    pub fn tombstones(&self) -> Result<Vec<StoredTombstone>, SqliteError> {
        rows::tombstones(&self.connection())
    }

    /// Every stored conflict, oldest first.
    ///
    /// # Errors
    ///
    /// Returns a [`SqliteError`] when the database cannot be read.
    pub fn conflicts(&self) -> Result<Vec<StoredConflict>, SqliteError> {
        rows::conflicts(&self.connection())
    }
}

#[cfg(test)]
mod tests;

impl JournalStore for Journal {
    fn writes(&self) -> usize {
        Journal::writes(self)
    }

    fn apply(&self, ops: &[Op]) -> Result<(), JournalError> {
        Ok(Journal::apply(self, ops)?)
    }

    fn items(&self) -> Result<Vec<JournalItem>, JournalError> {
        Ok(Journal::items(self)?)
    }

    fn item_by_remote(&self, id: &RemoteId) -> Result<Option<JournalItem>, JournalError> {
        Ok(Journal::item_by_remote(self, id)?)
    }

    fn new_item_at(&self, path: &ItemPath) -> Result<Option<JournalItem>, JournalError> {
        Ok(Journal::new_item_at(self, path)?)
    }

    fn tombstone_of(&self, id: &RemoteId) -> Result<Option<StoredTombstone>, JournalError> {
        Ok(Journal::tombstone_of(self, id)?)
    }

    fn anchor(&self) -> Result<Option<StoredAnchor>, JournalError> {
        Ok(Journal::anchor(self)?)
    }

    fn tombstones(&self) -> Result<Vec<StoredTombstone>, JournalError> {
        Ok(Journal::tombstones(self)?)
    }

    fn conflicts(&self) -> Result<Vec<StoredConflict>, JournalError> {
        Ok(Journal::conflicts(self)?)
    }
}
