//! The SQLite files syncd keeps (io: rusqlite and the file system).
//!
//! - `journal`: [`Journal`], the journal of one dataset of one account, the
//!   `sync_engine::journal::JournalStore` the engine runs over; every change one transaction.
//! - `drive`: [`DriveJournal`], the listing file of one account's drive that Files shows (entries
//!   by id, the feed's anchor and the folder tree), each page written in one transaction.
//!
//! `Journal::open_unflushed` (feature `testing`) skips the disk flush for tests that make
//! thousands of commits.

pub mod drive;
pub mod journal;

pub use drive::{Applied, DriveEntry, DriveJournal};
pub use journal::{Journal, SqliteError, prepare_with};

#[cfg(test)]
mod testing;
