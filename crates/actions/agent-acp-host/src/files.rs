//! The file system the agent's `fs/*` calls reach, behind a seam. `OsFiles` is the real one, std
//! only; a test uses `FakeFiles` (in `fake`). The seam answers three things: where a path really
//! leads (links followed), a bounded read, and a write that hands back what it replaced so the
//! undo journal can keep it.
//!
//! `OsFiles` checks a second time after it opens: the descriptor's own target (`/proc/self/fd`)
//! must still lie inside the session's real directory, so a link swapped in between the check and
//! the open (the agent can write in its directory and make links) is caught before any byte is
//! read or changed.

use super::workdir::{Workdir, WorkdirError};
use base_path::AbsPath;
use base_xdg::Xdg;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

/// The most a read returns.
pub const MAX_READ: usize = 2 * 1024 * 1024;
/// The most a write accepts.
pub const MAX_WRITE: usize = 4 * 1024 * 1024;

/// Why a file call failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FileError {
    /// There is no such file.
    #[error("no such file")]
    NotFound,
    /// Too large to read or write through ACP.
    #[error("the file is too large")]
    TooBig,
    /// Not UTF-8 text.
    #[error("the file is not text")]
    NotText,
    /// The file system refused.
    #[error("the file system refused")]
    Refused,
    /// What the path leads to changed between the check and the open.
    #[error("the path changed under the call")]
    Moved,
    /// A directory, a pipe, a device or a socket: only a regular file is read.
    #[error("not a regular file")]
    NotRegular,
}

/// The seam.
pub trait Files: Send {
    /// Where `path` really leads, links followed. A missing last part is allowed (a new file):
    /// the nearest existing parent is resolved and the rest appended.
    ///
    /// # Errors
    /// Returns a [`FileError`] when no part of the path can be resolved or the result is not an
    /// absolute path.
    fn real(&self, path: &AbsPath) -> Result<AbsPath, FileError>;

    /// `asked`, a directory inside `scope`, as a confined directory a sandbox may write. This
    /// default resolves both through `real` and applies the pure rules, which suits a file system
    /// without links to walk; `OsFiles` looks at every part and refuses links and the home folder.
    ///
    /// # Errors
    /// Returns a [`WorkdirError`] when either path cannot be resolved or `asked` is not a folder
    /// the rules allow inside `scope`.
    fn workdir(&self, scope: &AbsPath, asked: &AbsPath) -> Result<Workdir, WorkdirError> {
        let real_scope = self.real(scope).map_err(|error| {
            tracing::debug!(%error, "the session's folder cannot be resolved");
            WorkdirError::NotFolder
        })?;
        let real = self.real(asked).map_err(|error| {
            tracing::debug!(%error, "the working folder cannot be resolved");
            WorkdirError::NotFolder
        })?;
        Workdir::from_resolved(&real_scope, real, None)
    }

    /// The text of the file at `real`, which must still lie inside `within`.
    ///
    /// # Errors
    /// Returns a [`FileError`] when the file is missing, not a regular file, too large, not text,
    /// moved out of `within`, or the file system refuses.
    fn read(&mut self, real: &AbsPath, within: &AbsPath) -> Result<String, FileError>;

    /// Replaces (or creates) the file at `real`, which must still lie inside `within`. Returns
    /// the text it replaced, `None` for a new file.
    ///
    /// # Errors
    /// Returns a [`FileError`] when the content is too large, the file cannot be read back or
    /// written, it moved out of `within`, or the file system refuses.
    fn write(
        &mut self,
        real: &AbsPath,
        within: &AbsPath,
        content: &str,
    ) -> Result<Option<String>, FileError>;

    /// Removes the file at `real`, which must still lie inside `within`: what undoing a write
    /// that created the file does.
    ///
    /// # Errors
    /// Returns a [`FileError`] when the file is missing, moved out of `within`, or the file
    /// system refuses.
    fn remove(&mut self, real: &AbsPath, within: &AbsPath) -> Result<(), FileError>;
}

/// The real file system.
#[derive(Debug, Clone, Copy, Default)]
pub struct OsFiles;

fn abs(path: &Path) -> Result<AbsPath, FileError> {
    path.to_str()
        .and_then(|t| AbsPath::parse(t).ok())
        .ok_or(FileError::Refused)
}

fn fault(error: &std::io::Error) -> FileError {
    match error.kind() {
        std::io::ErrorKind::NotFound => FileError::NotFound,
        _ => FileError::Refused,
    }
}

/// Resolves the deepest existing ancestor of `path` and appends the rest.
fn resolve(path: &Path) -> Result<PathBuf, FileError> {
    let mut rest = Vec::new();
    let mut here = path.to_path_buf();
    loop {
        if let Ok(real) = here.canonicalize() {
            return Ok(rest.iter().rev().fold(real, |acc, part| acc.join(part)));
        }
        let name = here.file_name().map(ToOwned::to_owned);
        let (Some(name), Some(parent)) = (name, here.parent()) else {
            return Err(FileError::NotFound);
        };
        rest.push(name);
        here = parent.to_path_buf();
    }
}

/// `O_NONBLOCK` where it is known: opening a pipe without it waits for a writer, and a pipe the
/// agent made in its own directory would hold the caller (and the performer's lock) for good.
/// Elsewhere 0: the regular-file check before the open is then the only guard.
#[cfg(all(
    target_os = "linux",
    any(
        target_arch = "x86",
        target_arch = "x86_64",
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "riscv64",
        target_arch = "powerpc64",
        target_arch = "s390x",
        target_arch = "loongarch64"
    )
))]
const NONBLOCK: i32 = 0o4000;
#[cfg(all(unix, target_vendor = "apple"))]
const NONBLOCK: i32 = 0x0004;
#[cfg(not(any(
    all(
        target_os = "linux",
        any(
            target_arch = "x86",
            target_arch = "x86_64",
            target_arch = "arm",
            target_arch = "aarch64",
            target_arch = "riscv64",
            target_arch = "powerpc64",
            target_arch = "s390x",
            target_arch = "loongarch64"
        )
    ),
    all(unix, target_vendor = "apple")
)))]
const NONBLOCK: i32 = 0;

/// Opens `path` for reading without waiting for anyone on the other end.
fn open_for_read(path: &str) -> std::io::Result<File> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        OpenOptions::new()
            .read(true)
            .custom_flags(NONBLOCK)
            .open(path)
    }
    #[cfg(not(unix))]
    {
        File::open(path)
    }
}

/// Whether the open descriptor still points inside `within`.
#[cfg(unix)]
fn still_inside(file: &File, within: &AbsPath) -> Result<(), FileError> {
    use std::os::fd::AsRawFd;
    let target =
        std::fs::read_link(format!("/proc/self/fd/{}", file.as_raw_fd())).map_err(|error| {
            tracing::debug!(%error, "the open file's path cannot be read");
            FileError::Moved
        })?;
    let target = abs(&target).map_err(|error| {
        tracing::debug!(%error, "the open file's path is not absolute text");
        FileError::Moved
    })?;
    if within.covers(&target) {
        Ok(())
    } else {
        Err(FileError::Moved)
    }
}

/// Where there is no `/proc/self/fd`, the check cannot be made, so the call is refused.
#[cfg(not(unix))]
fn still_inside(_file: &File, _within: &AbsPath) -> Result<(), FileError> {
    Err(FileError::Moved)
}

fn read_text(file: &mut File) -> Result<String, FileError> {
    let mut bytes = Vec::new();
    Read::by_ref(file)
        .take(MAX_READ as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| fault(&e))?;
    if bytes.len() > MAX_READ {
        return Err(FileError::TooBig);
    }
    String::from_utf8(bytes).map_err(|error| {
        tracing::debug!(%error, "the file is not utf-8");
        FileError::NotText
    })
}

impl Files for OsFiles {
    fn workdir(&self, scope: &AbsPath, asked: &AbsPath) -> Result<Workdir, WorkdirError> {
        let xdg = Xdg::process();
        Workdir::resolve(scope, asked, xdg.home())
    }

    fn real(&self, path: &AbsPath) -> Result<AbsPath, FileError> {
        let resolved = resolve(Path::new(path.as_str()))?;
        // `Component` keeps only normal parts: a resolved path has no `..` left.
        debug_assert!(
            resolved
                .components()
                .all(|c| !matches!(c, Component::ParentDir))
        );
        abs(&resolved)
    }

    fn read(&mut self, real: &AbsPath, within: &AbsPath) -> Result<String, FileError> {
        // Looked at before the open, so a pipe is refused where the open cannot be made not to
        // wait; and again on the descriptor, which is what is read.
        if !std::fs::metadata(real.as_str())
            .map_err(|e| fault(&e))?
            .is_file()
        {
            return Err(FileError::NotRegular);
        }
        let mut file = open_for_read(real.as_str()).map_err(|e| fault(&e))?;
        still_inside(&file, within)?;
        let meta = file.metadata().map_err(|e| fault(&e))?;
        if !meta.is_file() {
            return Err(FileError::NotRegular);
        }
        if meta.len() > MAX_READ as u64 {
            return Err(FileError::TooBig);
        }
        read_text(&mut file)
    }

    fn write(
        &mut self,
        real: &AbsPath,
        within: &AbsPath,
        content: &str,
    ) -> Result<Option<String>, FileError> {
        if content.len() > MAX_WRITE {
            return Err(FileError::TooBig);
        }
        let existed = Path::new(real.as_str()).exists();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(real.as_str())
            .map_err(|e| fault(&e))?;
        if let Err(outside) = still_inside(&file, within) {
            // A link swapped in after the check made `open` create the file somewhere else: an
            // empty file this call made is not left behind.
            if !existed && file.metadata().is_ok_and(|m| m.len() == 0) {
                let _ = std::fs::remove_file(real.as_str());
            }
            return Err(outside);
        }
        let before = if existed {
            Some(read_text(&mut file)?)
        } else {
            None
        };
        file.set_len(0).map_err(|e| fault(&e))?;
        // `read_text` moved the position; a write starts at the front.
        std::io::Seek::rewind(&mut file).map_err(|e| fault(&e))?;
        file.write_all(content.as_bytes()).map_err(|e| fault(&e))?;
        Ok(before)
    }

    fn remove(&mut self, real: &AbsPath, within: &AbsPath) -> Result<(), FileError> {
        let file = open_for_read(real.as_str()).map_err(|e| fault(&e))?;
        still_inside(&file, within)?;
        std::fs::remove_file(real.as_str()).map_err(|e| fault(&e))
    }
}
