//! The bearer token's file of an attached engine (vLLM's `--api-key`): where it is and why one was
//! not used. Reading it is `inference-fs`'s (`read_key`), at each connect.
//!
//! The file is the person's own and private to them: a file any other user could read is refused,
//! and so is one another user owns. The token is held in `SecretText` (whose `Debug` says nothing)
//! and goes nowhere but the `Authorization` header: no log line, error or `Debug` output of this
//! module holds it.

use std::path::{Path, PathBuf};

/// Why a key file was not used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KeyFileProblem {
    /// There is no file there.
    #[error("the key file is missing")]
    Missing,
    /// It could not be opened or read.
    #[error("the key file cannot be read: {0:?}")]
    Unreadable(std::io::ErrorKind),
    /// It is not text.
    #[error("the key file is not text")]
    NotText(#[source] std::string::FromUtf8Error),
    /// It is not a regular file.
    #[error("the key file is not a regular file")]
    NotAFile,
    /// Group or others have some access to it.
    #[error("the key file is readable by others (mode {mode:04o}); it must be 0600")]
    NotPrivate {
        /// Its permission bits.
        mode: u32,
    },
    /// Another user owns it.
    #[error("the key file belongs to another user")]
    NotYours,
    /// It holds nothing, or more than a token.
    #[error("the key file holds no usable token")]
    Unusable,
}

/// A key file's path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyFile(PathBuf);

impl KeyFile {
    /// The key file at `path`.
    #[must_use]
    pub fn at(path: PathBuf) -> Self {
        Self(path)
    }

    /// Where it is.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}
