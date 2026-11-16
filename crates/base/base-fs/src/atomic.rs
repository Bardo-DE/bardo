//! The options of the one atomic writer, and a small file kept with its backup.
//!
//! A write goes to a staging file in the target's own directory (`.<name>.<pid>.<n>.tmp`, a dot
//! file whose name no other save, in this process or another, is using), is written whole and
//! synced to disk, is renamed over the target, and the directory is then synced so the rename
//! survives a power cut. A failure before the rename leaves the old file whole and removes the
//! staging file. Blocking calls only; the files are small.
//!
//! The mechanism is [`crate::write_atomic`]'s, shared and not copied: a symlink at the target is
//! followed and its target replaced, an existing file keeps its permissions, and writers of one
//! path in this process take turns. [`AtomicWrite`] only chooses what `write_atomic` fixes: the
//! mode of a new file, and whether missing parent folders are made.
//!
//! [`AtomicWrite`] is the writer; [`AtomicFile`] is one small file kept with the copy from before
//! its last change (`<name>.bak`), as the registry is.

use crate::write::{self, NewFileMode, directory_of};
use crate::{link, lock};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub use crate::write::staging_beside;

/// A way of writing a file atomically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtomicWrite {
    /// Whether missing directories above the target are made; when not, a write into a directory
    /// that is gone fails with `NotFound` and brings nothing back.
    create_parents: bool,
    /// The permissions of a new file; a file that exists keeps its own.
    new_file: NewFileMode,
}

impl AtomicWrite {
    /// Owner-only files, missing directories made: credentials, settings, ledgers.
    pub const PRIVATE: Self = Self {
        create_parents: true,
        new_file: NewFileMode::OwnerOnly,
    };

    /// Files with the default permissions, missing directories made: data other programs read.
    pub const SHARED: Self = Self {
        create_parents: true,
        new_file: NewFileMode::Umask,
    };

    /// The same writer, but it never makes a directory (a write into a missing one fails with `NotFound`).
    #[must_use]
    pub const fn in_existing_dir(self) -> Self {
        Self {
            create_parents: false,
            ..self
        }
    }

    /// Replaces `target` with `bytes`, atomically.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of staging or committing the write.
    pub fn write(&self, target: &Path, bytes: &[u8]) -> io::Result<()> {
        self.publish(target, |file| file.write_all(bytes))
    }

    /// Replaces `target` with the contents of `source`, the same way.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of reading `source` or of staging or committing the write.
    pub fn copy(&self, source: &Path, target: &Path) -> io::Result<()> {
        self.publish(target, |file| {
            io::copy(&mut std::fs::File::open(source)?, file).map(|_| ())
        })
    }

    /// The first half of a write: `bytes` on disk and synced under a staging name beside
    /// `target`, which it returns. Until [`AtomicWrite::commit`] the target is untouched.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of resolving `target` or of writing and syncing the staged file.
    pub fn stage(&self, target: &Path, bytes: &[u8]) -> io::Result<PathBuf> {
        let target = link::resolve(target)?;
        lock::with(&target, || {
            self.stage_resolved(&target, |file| file.write_all(bytes))
        })
    }

    /// The second half: the staged file replaces `target` and the directory entry is made
    /// durable. The staged file is removed if that fails.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of resolving `target`, renaming the staged file or syncing the
    /// directory.
    pub fn commit(&self, staged: &Path, target: &Path) -> io::Result<()> {
        let target = link::resolve(target)?;
        lock::with(&target, || write::commit(staged, &target))
    }

    fn publish(
        self,
        target: &Path,
        fill: impl FnOnce(&mut std::fs::File) -> io::Result<()>,
    ) -> io::Result<()> {
        let target = link::resolve(target)?;
        lock::with(&target, || {
            let staged = self.stage_resolved(&target, fill)?;
            write::commit(&staged, &target)
        })
    }

    fn stage_resolved(
        self,
        target: &Path,
        fill: impl FnOnce(&mut std::fs::File) -> io::Result<()>,
    ) -> io::Result<PathBuf> {
        if self.create_parents {
            std::fs::create_dir_all(directory_of(target))?;
        }
        // An owner-only file is made owner-only again on every write (a credential found at 0644 is
        // tightened); a shared one keeps the mode it has.
        write::stage(
            target,
            self.new_file,
            self.new_file == NewFileMode::Umask,
            fill,
        )
    }
}

/// A file `name` in `dir`, owner-only, kept with the copy from before its last change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomicFile {
    dir: PathBuf,
    name: String,
}

impl AtomicFile {
    /// The file `name` in `dir` (the directory is created on the first save).
    pub fn new(dir: PathBuf, name: impl Into<String>) -> Self {
        Self {
            dir,
            name: name.into(),
        }
    }

    /// The file at `path` (its directory is created on the first save). `None` for a path with no
    /// file name.
    #[must_use]
    pub fn at(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().into_owned();
        Some(Self::new(directory_of(path).to_path_buf(), name))
    }

    /// The file.
    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.dir.join(&self.name)
    }

    /// The copy of the file from before its last change.
    #[must_use]
    pub fn backup_path(&self) -> PathBuf {
        self.dir.join(format!("{}.bak", self.name))
    }

    /// The file's bytes; `None` when there is no file.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of reading the file, other than its being absent.
    pub fn read(&self) -> io::Result<Option<Vec<u8>>> {
        match std::fs::read(self.path()) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Replaces the file with `text`, atomically, keeping the one it replaces as the backup.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of staging the text or of replacing the file or keeping its backup.
    pub fn write(&self, text: &str) -> io::Result<()> {
        let staged = self.stage(text)?;
        let kept = self.keep_previous();
        if let Err(e) = kept {
            let _ = std::fs::remove_file(&staged);
            return Err(e);
        }
        AtomicWrite::PRIVATE.commit(&staged, &self.path())
    }

    /// A staging name no other save, in this process or another, is using.
    #[must_use]
    pub fn staging(&self) -> PathBuf {
        staging_beside(&self.path())
    }

    /// The first half of a save: the whole document on disk and synced, under a staging name of
    /// its own, which it returns. Until the commit the file is untouched.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of resolving the path or of writing and syncing the staged file.
    pub fn stage(&self, text: &str) -> io::Result<PathBuf> {
        AtomicWrite::PRIVATE.stage(&self.path(), text.as_bytes())
    }

    /// Makes the backup the file as it is now, atomically. Nothing to keep on the first save.
    fn keep_previous(&self) -> io::Result<()> {
        match std::fs::read(self.path()) {
            Ok(bytes) => AtomicWrite::PRIVATE.write(&self.backup_path(), &bytes),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests;
