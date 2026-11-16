//! The one error `anyview-store` returns, with variants a caller acts on: a missing file is not
//! one (it is an empty store), a damaged file is told apart from a failing disk.

use std::path::PathBuf;

/// What the store was doing when a disk operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreOp {
    /// Reading a file.
    Read,
}

/// Why the store could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreError {
    /// The disk refused an operation. A reader treats the store as unavailable.
    #[error("{op:?} failed on {path:?}: {kind}")]
    Io {
        /// What was being done.
        op: StoreOp,
        /// The file or directory.
        path: PathBuf,
        /// What the operating system reported.
        kind: std::io::ErrorKind,
    },
    /// A file exists and is not what the store writes: cut short, edited by hand, or from a
    /// program that is not this one.
    #[error("{path:?} is not a valid store file: {reason}")]
    Corrupt {
        /// The file.
        path: PathBuf,
        /// The parser's description.
        reason: String,
    },
}
