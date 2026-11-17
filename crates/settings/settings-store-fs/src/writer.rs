//! Changing one key of a settings file in place.
//!
//! A program that does not own the file (the Settings app, an agent) or owns only a field of it
//! (a consent flag) must not load the whole document and save it back: a change made in between
//! would be lost. [`SettingsWriter::patch`] reads the file under its lock, sets the one key in
//! the parsed text and writes the result atomically, so every comment, every other key and the
//! person's formatting stay as they were. [`SettingsWriter::unset`] takes a key out the same way.

use crate::error::SettingsError;
use crate::store::Store;
use crate::text::SETTINGS_CAP;
use base_fs::{NewFileMode, UpdateError, update_atomic};
use settings_store::{Edit, SettingsDoc, apply_edits};
use settings_types::schema::KeyPath;
use std::path::{Path, PathBuf};

/// The writer of one TOML settings file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsWriter {
    path: PathBuf,
}

impl SettingsWriter {
    /// A writer for the file at `path`. The file need not exist yet.
    #[must_use]
    pub fn new(path: PathBuf) -> SettingsWriter {
        SettingsWriter { path }
    }

    /// A writer for the file `D` is stored in inside `store`'s directory.
    ///
    /// # Errors
    ///
    /// Returns a [`SettingsError`] when the store has no config directory for `D`.
    pub fn of<D: SettingsDoc>(store: &Store) -> Result<SettingsWriter, SettingsError> {
        store
            .path::<D>()
            .map(SettingsWriter::new)
            .ok_or_else(|| store.no_config_dir())
    }

    /// The file this writes.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Sets `key` (a dotted path) to `value`, making the tables on the way, and leaves every
    /// other byte of the file as it was. The key keeps its place, spacing and trailing comment
    /// when it is already there. A table value is written as an inline table
    /// (`consent = { given = 1760000000 }`). The read, the edit and the write happen under the
    /// file's lock ([`base_fs::update_atomic`]): a crash cannot leave half a file and a
    /// directory watch sees one rename.
    ///
    /// A file that is there and cannot be used, or is not valid TOML, or a key under a value that
    /// is not a table, is left alone and reported. A directory that is not there is made.
    ///
    /// # Errors
    ///
    /// Returns a [`SettingsError`] when the file cannot be read, is not valid TOML, or cannot take
    /// the edit.
    pub fn patch(&self, key: &KeyPath, value: &toml::Value) -> Result<(), SettingsError> {
        self.patch_all(&[(key.clone(), value.clone())])
    }

    /// Applies every edit, in order, under one lock and in one atomic write. If any edit fails
    /// nothing is written.
    ///
    /// # Errors
    ///
    /// Returns a [`SettingsError`] when the file cannot be read, is not valid TOML, or cannot take
    /// the edits.
    pub fn patch_all(&self, edits: &[(KeyPath, toml::Value)]) -> Result<(), SettingsError> {
        let edits: Vec<Edit> = edits
            .iter()
            .map(|(key, value)| Edit::Set(key.clone(), value.clone()))
            .collect();
        self.apply(&edits)
    }

    /// Takes `key` (a dotted path) out of the file, so the program reads its default again,
    /// and leaves every other byte as it was. A key that is not there, or a table on its path
    /// that is not there, is already unset: the file stays as it is. The tables on the way stay,
    /// even when the key was their last. Same lock and atomic write as [`patch`](Self::patch).
    ///
    /// # Errors
    ///
    /// Returns a [`SettingsError`] when the file cannot be read, is not valid TOML, or cannot take
    /// the edit.
    pub fn unset(&self, key: &KeyPath) -> Result<(), SettingsError> {
        self.apply(&[Edit::Unset(key.clone())])
    }

    /// Applies every edit, in order, under one lock and in one atomic write. If any edit fails
    /// nothing is written.
    ///
    /// # Errors
    ///
    /// Returns a [`SettingsError`] when the file cannot be read, is not valid TOML, or cannot take
    /// the edits.
    pub fn apply(&self, edits: &[Edit]) -> Result<(), SettingsError> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|source| SettingsError::Io {
                path: dir.to_path_buf(),
                source,
            })?;
        }
        let update = update_atomic(&self.path, SETTINGS_CAP, NewFileMode::Umask, |existing| {
            apply_edits(existing.as_deref().unwrap_or_default(), edits).map_err(|source| {
                SettingsError::Patch {
                    path: self.path.clone(),
                    source,
                }
            })
        });
        match update {
            Ok(()) => Ok(()),
            Err(UpdateError::Read(error)) => Err(error.into()),
            Err(UpdateError::Edit(error)) => Err(error),
            Err(UpdateError::Write(error)) => Err(error.into()),
        }
    }
}
