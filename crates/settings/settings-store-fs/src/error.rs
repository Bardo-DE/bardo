//! What can go wrong reading or writing settings. Reading never fails the caller (a damaged
//! file is reported in [`settings_store::Loaded`]); writing can.

use settings_store::{EditError, EncodeError};
use std::path::PathBuf;

/// A settings write that did not happen.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// A file or directory could not be read, written or renamed.
    #[error("{path}: {source}")]
    Io {
        /// Which path.
        path: PathBuf,
        /// Why.
        source: std::io::Error,
    },
    /// The value could not be written in the document's format.
    #[error(transparent)]
    Encode(#[from] EncodeError),
    /// A file there could not be read as text under the size cap (see [`base_fs::ReadError`]);
    /// it is left as it was.
    #[error(transparent)]
    Read(#[from] base_fs::ReadError),
    /// A file could not take the edit: it is not valid TOML, or the key sits inside a value that
    /// is not a table. It is left as it was.
    #[error("{path}: {source}")]
    Patch {
        /// Which file.
        path: PathBuf,
        /// What is wrong with it.
        source: EditError,
    },
    /// The directory watch could not be started.
    #[error("watching settings: {0}")]
    Watch(#[from] notify::Error),
    /// The config root has no directory for the program (no `$XDG_CONFIG_HOME` and no `$HOME`).
    #[error("no config directory for {app}")]
    NoConfigDir {
        /// The program's directory name.
        app: &'static str,
    },
}

impl From<base_fs::WriteError> for SettingsError {
    fn from(error: base_fs::WriteError) -> Self {
        SettingsError::Io {
            path: error.path,
            source: error.source,
        }
    }
}
