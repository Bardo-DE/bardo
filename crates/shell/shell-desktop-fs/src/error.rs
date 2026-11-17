//! What a file helper can fail at.

use std::path::{Path, PathBuf};

/// A file or folder that could not be read, written or removed.
#[derive(Debug, thiserror::Error)]
#[error("{path}: {source}")]
pub struct FileError {
    /// The file or folder.
    pub path: PathBuf,
    /// What the system said.
    #[source]
    pub source: std::io::Error,
}

impl FileError {
    pub(crate) fn at(path: &Path, source: std::io::Error) -> FileError {
        FileError {
            path: path.to_path_buf(),
            source,
        }
    }
}
