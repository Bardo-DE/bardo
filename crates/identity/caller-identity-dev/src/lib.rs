//! Developer rows: a checkout-built stand-in has an image its author can write and no real unit,
//! so the caller-identity judgement can never give it a service's role; a developer row says
//! "this image is that service" and nothing else.
//!
//! **The risk, plainly.** Rows from a file a person owns let that person's programs take a
//! service's role. They are safe only because the standing they produce, `Proof::Dev`, is below
//! every real proof and a consumer accepts it only when it asks for it
//! ([`caller_identity_core::Requirement::dev`]), in its own debug builds. This crate is the only
//! way to make developer rows, and no shipped program depends on it: the desktop gate greps
//! `cargo tree` for it, as it does for the `fake` seams. Consumers reach it through a feature of
//! their own (`debug`, `dev-callers`) that is off by default.
//!
//! The rows are added to a table by [`DevRows::added_to`], which marks each as a developer row
//! through [`caller_identity_core::Table::with_dev_rows`]; the rows are never a [`Table`] a caller
//! could merge as if they were shipped.

use caller_identity_core::{ProcFs, Table, TableError, Uid};
use caller_identity_linux::{RealProc, TABLE_FILE_CAP};
use std::io;
use std::os::unix::fs::MetadataExt as _;
use std::path::Path;

/// Group and others may write: a file with either bit is not one person's.
const SHARED_WRITE: u32 = 0o022;

/// Why a developer rows file was not taken.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DevFileError {
    /// The file could not be looked at or read, or is over the size cap.
    #[error("{path}: could not be read")]
    Unreadable {
        /// The file.
        path: String,
        /// The cause.
        #[source]
        source: io::Error,
    },
    /// The file is a link or not a plain file.
    #[error("{path}: not a plain file")]
    NotAFile {
        /// The file.
        path: String,
    },
    /// The file is not the person's own, or group or others can write it.
    #[error("{path}: not the person's own file, or others can write it")]
    NotTheirs {
        /// The file.
        path: String,
    },
    /// The file is not a table.
    #[error("{path}: {source}")]
    Table {
        /// The file.
        path: String,
        /// What is wrong with it.
        #[source]
        source: TableError,
    },
}

/// What a path is, without following a final link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Node {
    File,
    Other,
}

/// The rows of a developer rows file, not yet part of any table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DevRows(Table);

impl DevRows {
    /// No rows.
    #[must_use]
    pub fn empty() -> DevRows {
        DevRows(Table::empty())
    }

    /// The rows in `path`. The file must be a plain file (a link is refused), owned by `person` or
    /// by root, not writable by group or others, and within the caller table's size cap; it
    /// must parse as a table. Any other state fails the whole load.
    ///
    /// # Errors
    ///
    /// Returns a [`DevFileError`] when the file is missing or unreadable, is not a plain file, is
    /// not owned by `person` or root, is writable by group or others, is over the size cap or is
    /// not a table.
    pub fn load(path: &Path, person: Uid) -> Result<DevRows, DevFileError> {
        let fs = RealProc::system();
        let meta = std::fs::symlink_metadata(path).map_err(|e| unreadable(path, e))?;
        let node = if meta.file_type().is_file() {
            Node::File
        } else {
            Node::Other
        };
        let owner = Uid::new(fs.owners().owner(meta.uid()));
        check(node, owner, meta.mode(), person, path)?;
        let bytes = fs
            .read_capped(path, TABLE_FILE_CAP)
            .map_err(|e| unreadable(path, e))?;
        let text = String::from_utf8(bytes)
            .map_err(|e| unreadable(path, io::Error::new(io::ErrorKind::InvalidData, e)))?;
        Table::from_toml(&text)
            .map(DevRows)
            .map_err(|source| DevFileError::Table {
                path: path.display().to_string(),
                source,
            })
    }

    /// `shipped` with these rows added as developer rows: after its own, never replacing one, and
    /// never taking a unit the shipped table names.
    #[must_use]
    pub fn added_to(self, shipped: Table) -> Table {
        shipped.with_dev_rows(self.0)
    }

    /// How many rows there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.rows().len()
    }

    /// Whether there are no rows.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.rows().is_empty()
    }
}

fn unreadable(path: &Path, source: io::Error) -> DevFileError {
    DevFileError::Unreadable {
        path: path.display().to_string(),
        source,
    }
}

/// Whether a file of this kind, owner and mode is the person's own plain file.
fn check(node: Node, owner: Uid, mode: u32, person: Uid, path: &Path) -> Result<(), DevFileError> {
    if node != Node::File {
        return Err(DevFileError::NotAFile {
            path: path.display().to_string(),
        });
    }
    let theirs = owner == person || owner == Uid::ROOT;
    if !theirs || mode & SHARED_WRITE != 0 {
        return Err(DevFileError::NotTheirs {
            path: path.display().to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt as _;

    const ROWS: &str = "[[peer]]\napp = \"org.bardo.Accounts\"\nexe = [\"/home/p/proj/target/debug/standin\"]\nrole = \"accounts\"\n";
    const SHIPPED: &str = "[[peer]]\napp = \"org.bardo.Shell\"\nunit = \"bardo-shell.service\"\nexe = [\"/usr/bin/bardo-shell\"]\nrole = \"shell\"\n";
    const PERSON: Uid = Uid::new(1000);

    fn path() -> &'static Path {
        Path::new("/home/p/scratch/dev-callers.toml")
    }

    #[test]
    fn only_the_persons_or_roots_plain_file_nobody_else_can_write_is_taken() {
        let cases = [
            ("the person's own", Node::File, 1000, 0o600, true),
            ("root's", Node::File, 0, 0o644, true),
            ("someone else's", Node::File, 1001, 0o600, false),
            ("group-writable", Node::File, 1000, 0o660, false),
            ("others-writable", Node::File, 1000, 0o602, false),
            ("a link or a folder", Node::Other, 1000, 0o600, false),
        ];
        for (name, node, owner, mode, taken) in cases {
            let got = check(node, Uid::new(owner), mode, PERSON, path());
            assert_eq!(got.is_ok(), taken, "{name}");
        }
        assert!(matches!(
            check(Node::Other, PERSON, 0o600, PERSON, path()),
            Err(DevFileError::NotAFile { .. })
        ));
        assert!(matches!(
            check(Node::File, Uid::new(1001), 0o600, PERSON, path()),
            Err(DevFileError::NotTheirs { .. })
        ));
    }

    #[test]
    fn a_file_loads_as_developer_rows_that_come_after_the_shipped_table() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let file = dir.path().join("dev-callers.toml");
        std::fs::write(&file, ROWS).expect("the rows file");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).expect("its mode");
        let me = Uid::new(own_uid());
        let rows = DevRows::load(&file, me).expect("loads");
        assert_eq!(rows.len(), 1);
        let merged = rows.added_to(Table::from_toml(SHIPPED).expect("shipped"));
        let dev: Vec<bool> = merged.rows().iter().map(|r| r.is_dev()).collect();
        assert_eq!(dev, [false, true]);
    }

    #[test]
    fn a_link_a_loose_file_a_missing_file_or_a_bad_table_adds_nothing() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let me = Uid::new(own_uid());
        let loose = dir.path().join("loose.toml");
        std::fs::write(&loose, ROWS).expect("the rows file");
        std::fs::set_permissions(&loose, std::fs::Permissions::from_mode(0o666)).expect("mode");
        assert!(matches!(
            DevRows::load(&loose, me),
            Err(DevFileError::NotTheirs { .. })
        ));
        let target = dir.path().join("target.toml");
        std::fs::write(&target, ROWS).expect("the rows file");
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600)).expect("mode");
        let link = dir.path().join("link.toml");
        std::os::unix::fs::symlink(&target, &link).expect("a link");
        assert!(matches!(
            DevRows::load(&link, me),
            Err(DevFileError::NotAFile { .. })
        ));
        assert!(matches!(
            DevRows::load(&dir.path().join("none.toml"), me),
            Err(DevFileError::Unreadable { .. })
        ));
        let bad = dir.path().join("bad.toml");
        std::fs::write(&bad, "[[peer]]\napp = \"x\"\n").expect("the rows file");
        std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o600)).expect("mode");
        assert!(matches!(
            DevRows::load(&bad, me),
            Err(DevFileError::Table { .. })
        ));
    }

    fn own_uid() -> u32 {
        std::fs::metadata("/proc/self").map_or(0, |m| m.uid())
    }
}
