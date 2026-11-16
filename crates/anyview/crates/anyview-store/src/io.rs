//! The disk edge: whole-file reads of the history. Nothing here decides anything.

use crate::error::{StoreError, StoreOp};
use serde::de::DeserializeOwned;
use std::io;
use std::path::Path;

pub(crate) fn io_error(op: StoreOp, path: &Path, error: &io::Error) -> StoreError {
    StoreError::Io {
        op,
        path: path.to_path_buf(),
        kind: error.kind(),
    }
}

/// The most a store file may be: the history holds a few hundred paths and a record one, so a
/// file over this is not one the store wrote.
const STORE_FILE_CAP: base_fs::ReadCap = base_fs::ReadCap::mib(16);

/// The JSON in `path`, or `None` when the file (or a directory above it) does not exist.
/// Anything else that keeps it from being read is an error, never "absent": a FIFO or a folder at
/// the path, or a refused read, is `Io` (left as it is), and a file over [`STORE_FILE_CAP`] is
/// `Corrupt` (a writer moves it aside before writing a fresh one).
pub(crate) fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, StoreError> {
    let bytes = match base_fs::read_capped(path, STORE_FILE_CAP) {
        Ok(bytes) => bytes,
        Err(error) => return read_failed(path, error),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| StoreError::Corrupt {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })
}

// `ReadError` is `non_exhaustive`: a failure this crate cannot name yet is a refused read, so
// the file is left as it is.
fn read_failed<T>(path: &Path, error: base_fs::ReadError) -> Result<Option<T>, StoreError> {
    match error {
        base_fs::ReadError::NotFound { .. } => Ok(None),
        base_fs::ReadError::NotRegular { .. } => Err(io_error(
            StoreOp::Read,
            path,
            &io::Error::from(io::ErrorKind::InvalidInput),
        )),
        base_fs::ReadError::TooBig { .. } => Err(StoreError::Corrupt {
            path: path.to_path_buf(),
            reason: error.to_string(),
        }),
        base_fs::ReadError::Io { source, .. } => Err(io_error(StoreOp::Read, path, &source)),
        other @ base_fs::ReadError::NotText { .. } => {
            Err(io_error(StoreOp::Read, path, &io::Error::other(other)))
        }
    }
}
