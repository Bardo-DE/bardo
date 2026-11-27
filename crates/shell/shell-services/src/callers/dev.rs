//! A person's own rows (feature `dev-callers`, which only the bardo-shell binary's `debug` feature turns
//! on): the stand-ins a checkout builds, named in a file the person owns, are let in as the
//! services they stand for, at `Proof::Dev`, below every real proof. A release build has none of
//! this: the feature, the flag and `caller-identity-dev` are all absent.

use std::path::Path;

use caller_identity_dbus::caller_identity_core::Uid;
use caller_identity_dev::{DevFileError, DevRows};

use super::DesktopCallers;

impl DesktopCallers {
    /// These callers, with the developer rows in `path` added after the shipped ones.
    ///
    /// A row never takes a unit the shipped table names. The file must be a plain file of this
    /// user's (or root's) that no one else can write.
    ///
    /// # Errors
    ///
    /// Returns an error, and adds nothing, if the file is not such a file or cannot be loaded.
    pub fn with_dev_rows(self, path: &Path) -> Result<DesktopCallers, DevFileError> {
        let person = Uid::new(rustix::process::getuid().as_raw());
        let rows = DevRows::load(path, person)?;
        Ok(self.with_rows(rows))
    }

    /// These callers, with `rows` added after the shipped ones.
    #[must_use]
    pub fn with_rows(self, rows: DevRows) -> DesktopCallers {
        DesktopCallers::new(rows.added_to(self.table))
    }
}
