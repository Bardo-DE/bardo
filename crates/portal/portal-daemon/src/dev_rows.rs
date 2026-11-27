//! A person's own rows.
//!
//! They come with the feature `debug`, which turns on `caller-identity-dev`: the stand-ins a
//! checkout builds (a fake portal frontend, a fake shell), named in a file the person owns, are let
//! in as the programs they stand for, at `Proof::Dev`, below every real proof. A release build has
//! none of this: the feature, the environment variable and `caller-identity-dev` are all absent.

use std::path::Path;

use caller_identity_dbus::caller_identity_core::{Table, Uid};
use caller_identity_dev::{DevFileError, DevRows};

/// `shipped`, with the developer rows in `path` added after its own (a row never takes a unit the
/// shipped table names).
///
/// The file must be a plain file of this user's (or root's) that no one else can write, and its
/// rows must parse; otherwise the error says why and nothing is added.
///
/// # Errors
///
/// Returns the [`DevFileError`] that says why the file was not taken.
pub fn with_dev_rows(shipped: Table, path: &Path) -> Result<Table, DevFileError> {
    let person = Uid::new(rustix::process::getuid().as_raw());
    Ok(DevRows::load(path, person)?.added_to(shipped))
}
