//! A person's own rows (feature `dev-callers`, which only the `bardo-settings` binary's feature of the same
//! name turns on): the stand-ins a checkout builds, named in a file the person owns, are let in as the
//! services they stand for, at `Proof::Dev`, below every real proof. A release build has none of this:
//! the feature, the option and `caller-identity-dev` are all absent.

use caller_identity_dbus::caller_identity_core::{Table, Uid};
use caller_identity_dev::{DevFileError, DevRows};
use std::path::Path;
use std::sync::OnceLock;

/// The rows `bardo-settings --dev-callers` named, read once at start and added after the desktop's
/// table by every [`desktop_peers`](crate::desktop_peers).
static ROWS: OnceLock<DevRows> = OnceLock::new();

/// Why the person's rows could not be taken up.
#[derive(Debug, thiserror::Error)]
pub enum DevRowsError {
    /// The file is not the person's own plain file, or it does not parse.
    #[error("{0}")]
    Load(#[from] DevFileError),
    /// Rows were already taken up in this process.
    #[error("the developer rows were already read")]
    Twice,
}

/// The rows in `path`: a plain file of this user's (or root's) that group and others cannot write,
/// within the caller-identity library's size cap, every row a developer row. Anything else is an error.
///
/// # Errors
///
/// Returns the [`DevFileError`] that says why the file was not taken.
pub fn load_dev_rows(path: &Path) -> Result<DevRows, DevFileError> {
    let person = Uid::new(rustix::process::getuid().as_raw());
    DevRows::load(path, person)
}

/// Reads `path` and makes its rows part of every [`desktop_peers`](crate::desktop_peers) from now on.
/// Call it once, at start, before any peers are made.
///
/// # Errors
///
/// Returns a [`DevRowsError`] when the file is not taken or rows were already taken up.
pub fn use_dev_rows(path: &Path) -> Result<(), DevRowsError> {
    let rows = load_dev_rows(path)?;
    ROWS.set(rows).map_err(|_| DevRowsError::Twice)
}

/// The shipped table with the person's rows after it (a row never takes a unit the shipped table
/// names); the shipped table alone when no rows were taken up.
pub(crate) fn with_dev_rows(shipped: Table) -> Table {
    match ROWS.get() {
        Some(rows) => rows.clone().added_to(shipped),
        None => shipped,
    }
}
