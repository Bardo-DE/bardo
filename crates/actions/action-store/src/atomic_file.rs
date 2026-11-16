//! One durable way to replace a file, and one way to read a file that may not exist yet.
//!
//! A grant file holds the person's consent, so a crash or a failed write must leave the old
//! file or the new one, never half of one and never nothing. Both are the one implementation in
//! `base-fs`; this adds only the directory a first write needs and the "no such file is `None`"
//! shape the callers read.

use base_fs::{NewFileMode, ReadCap, ReadError};
use std::io::Error;
use std::path::Path;

/// The most a file this crate reads may hold.
const READ_CAP: ReadCap = ReadCap::mib(16);

/// Replaces the file at `path` with `bytes`, creating its directory when needed. When this
/// returns an error the file still holds what it held before.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    base_fs::write_atomic(path, bytes, NewFileMode::Umask).map_err(|why| why.source)
}

/// The text of the file at `path`, or `None` when there is no such file. Any other failure is
/// an error: an unreadable file is not an empty one.
pub fn read_optional(path: &Path) -> std::io::Result<Option<String>> {
    match base_fs::read_text_capped(path, READ_CAP) {
        Ok(text) => Ok(Some(text)),
        Err(ReadError::NotFound { .. }) => Ok(None),
        Err(ReadError::Io { source, .. }) => Err(source),
        Err(other) => Err(Error::other(other)),
    }
}
