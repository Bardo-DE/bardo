//! Changing a file's text where two writers cannot lose each other's change.

use crate::cap::ReadCap;
use crate::flock::Flock;
use crate::read::{ReadError, read_text_capped};
use crate::write::{NewFileMode, WriteError, directory_of, publish};
use crate::{link, lock};
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Why an update did not happen.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError<E> {
    /// The file is there and could not be read as text under the cap. Nothing was changed.
    #[error(transparent)]
    Read(ReadError),
    /// The caller's edit refused. Nothing was changed.
    #[error("{0}")]
    Edit(E),
    /// The lock could not be taken, or the new text could not be written. The old file is intact.
    #[error(transparent)]
    Write(WriteError),
}

/// Read the text of the file at `path`, let `edit` change it, and write the result back, while
/// holding the file's lock so that no other writer's change is lost in between.
///
/// `edit` gets the current text, or `None` when there is no file yet, and returns the whole new
/// text. The read goes through [`read_text_capped`] with `cap` (a file that exists and cannot be
/// read is [`UpdateError::Read`], never taken for a missing one and never written over), and the
/// write is [`write_atomic`](crate::write_atomic)'s: a unique staging file, `sync_all`, a
/// rename, an fsync of the directory, the file's mode kept (or `new_file` for a new one), a
/// symlink written through.
///
/// The lock is two locks: writers in this process queue on the file's path, and writers in other
/// programs on an `flock` of the sidecar `.<name>.lock` in the same directory (created with mode
/// 0600 and left in place). A holder that does not let go within a few seconds makes the update
/// fail with an error of kind `TimedOut`. A caller must not call `update_atomic` or
/// `write_atomic` for the same path from inside `edit`.
///
/// # Errors
///
/// Returns [`UpdateError`] when the file cannot be locked, read or written, or when `edit` returns
/// an error, in which case the file is left alone.
pub fn update_atomic<E>(
    path: &Path,
    cap: ReadCap,
    new_file: NewFileMode,
    edit: impl FnOnce(Option<String>) -> Result<String, E>,
) -> Result<(), UpdateError<E>> {
    let target = link::resolve(path).map_err(|source| write_failed(path, source))?;
    lock::with(&target, || under_lock(&target, cap, new_file, edit))
}

fn under_lock<E>(
    target: &Path,
    cap: ReadCap,
    new_file: NewFileMode,
    edit: impl FnOnce(Option<String>) -> Result<String, E>,
) -> Result<(), UpdateError<E>> {
    let sidecar = sidecar(target).map_err(|source| write_failed(target, source))?;
    let _held = Flock::exclusive(&sidecar).map_err(|source| write_failed(&sidecar, source))?;
    let current = match read_text_capped(target, cap) {
        Ok(text) => Some(text),
        Err(error) if error.is_missing() => None,
        Err(error) => return Err(UpdateError::Read(error)),
    };
    let text = edit(current).map_err(UpdateError::Edit)?;
    publish(target, new_file, |file| file.write_all(text.as_bytes()))
        .map_err(|source| write_failed(target, source))
}

fn write_failed<E>(path: &Path, source: io::Error) -> UpdateError<E> {
    UpdateError::Write(WriteError::at(path, source))
}

/// `.<name>.lock` in the directory of `target`.
fn sidecar(target: &Path) -> io::Result<PathBuf> {
    let name = target
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "the path has no file name"))?;
    let mut lock_name = OsString::from(".");
    lock_name.push(name);
    lock_name.push(".lock");
    Ok(directory_of(target).join(lock_name))
}
