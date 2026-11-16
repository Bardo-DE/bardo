//! The one way a file is read here: a plain file only, never more than a cap.
//!
//! An icon path can come from a client (through its `xdg_toplevel_icon` name, once it is a theme
//! name) or from a desktop entry a person can edit, so it may name `/dev/zero`, a pipe nobody
//! writes to, or a file that grows without end. `std::fs::read` would block or fill memory on
//! each of them.

use crate::IconError;
use base_fs::{ReadCap, ReadError, read_capped};
use std::path::Path;

/// The bytes of the plain file at `path`, at most `limit` of them.
///
/// Symbolic links are followed: icon themes are full of them, and what counts is what they lead
/// to. A file longer than `limit` is refused, not cut.
pub(crate) fn plain_file(path: &Path, limit: u64) -> Result<Vec<u8>, IconError> {
    read_capped(path, ReadCap::bytes(limit)).map_err(|error| match error {
        ReadError::NotRegular { path } => IconError::NotAFile { path },
        ReadError::TooBig { path, .. } => IconError::TooLarge { path },
        ReadError::NotFound { path } => IconError::Read {
            source: std::io::ErrorKind::NotFound.into(),
            path,
        },
        ReadError::Io { path, source } => IconError::Read { path, source },
        ReadError::NotText { path, source } => IconError::Read {
            path,
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, source),
        },
    })
}
