//! What was installed and how it was checked, kept beside the files. This is the record as a value
//! and as text; reading and writing the file is `bardo-agents`.

use crate::hash::Sha256Hex;
use crate::slug::Slug;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// How the install was checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Check {
    /// The registry gave the archive's digest and it matched.
    Declared,
    /// The registry gave none and this version was never installed before: the digest of what
    /// arrived is kept as the pin that every later install of it is compared with.
    FirstUse,
    /// The registry gave none; the digest matched the pin kept by the first install.
    Pinned,
    /// The package manager checked its own packages.
    PackageManager,
}

/// One installed version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// The registry id.
    pub id: String,
    /// The version.
    pub version: String,
    /// The program, relative to `files/`.
    pub command: String,
    /// Its arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Plain variables the registry says it wants.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// The archive's digest, when there was an archive.
    pub digest: Option<String>,
    /// How it was checked.
    pub check: Check,
}

/// Why a record was not read. Two faults are equal when they are the same kind of failure.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RecordError {
    /// Nothing installed there, or the file could not be read.
    #[error("that agent is not installed")]
    Missing(#[source] Arc<std::io::Error>),
    /// The text is not a record.
    #[error("the install record is damaged")]
    Damaged(#[source] toml::de::Error),
    /// The record names a program outside its directory, or a name or digest that is not valid.
    #[error("the install record names something it may not")]
    Invalid,
}

impl PartialEq for RecordError {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Eq for RecordError {}

/// The file name of the record.
pub const FILE: &str = "installed.toml";
/// The directory the files are in.
pub const FILES: &str = "files";

impl Record {
    /// The record as the text kept in `installed.toml`.
    ///
    /// # Errors
    ///
    /// Returns the TOML serialisation error when the record cannot be written.
    pub fn render(&self) -> Result<String, toml::ser::Error> {
        toml::to_string(self)
    }

    /// Reads the text of `installed.toml`; a record that names a program outside its directory, or
    /// a name or digest that is not one, is damaged.
    ///
    /// # Errors
    ///
    /// Returns [`RecordError::Damaged`] when the text is not a record, and [`RecordError::Invalid`] when
    /// it names a program outside its directory, or a name or digest that is not valid.
    pub fn parse(text: &str) -> Result<Self, RecordError> {
        let record: Self = toml::from_str(text).map_err(RecordError::Damaged)?;
        let named = Slug::parse(&record.id).is_ok() && Slug::parse(&record.version).is_ok();
        let stays = Path::new(&record.command)
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir));
        let digest = record
            .digest
            .as_deref()
            .is_none_or(|d| Sha256Hex::parse(d).is_ok());
        (named && stays && digest)
            .then_some(record)
            .ok_or(RecordError::Invalid)
    }

    /// The program's path, given the version's directory.
    #[must_use]
    pub fn command_in(&self, dir: &Path) -> PathBuf {
        dir.join(FILES).join(&self.command)
    }
}
