//! Writing a file so that a reader sees the old bytes or the new ones, never a mixture, and so
//! that the new ones survive a power cut. This is the one writer: [`stage`] and [`commit`] are its
//! two halves, [`publish`] the two together, and everything else in the crate that writes a file
//! ([`write_atomic`], [`update_atomic`](crate::update_atomic), [`AtomicWrite`](crate::atomic::AtomicWrite),
//! [`AtomicFile`](crate::atomic::AtomicFile)) goes through them.

use crate::{link, lock};
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// How many staging names are tried before the write gives up (each is unique per process, so
/// a clash means leftovers from another one).
const TEMP_TRIES: u32 = 32;

/// The numbers that make the staging names of one process distinct.
static STAGED: AtomicU64 = AtomicU64::new(0);

/// A write that did not happen.
#[derive(Debug, thiserror::Error)]
#[error("{path}: {source}")]
pub struct WriteError {
    /// The file being written (after any symlink was followed), or the directory that could not
    /// be synced.
    pub path: PathBuf,
    /// Why.
    pub source: io::Error,
}

impl WriteError {
    pub(super) fn at(path: &Path, source: io::Error) -> Self {
        WriteError {
            path: path.to_path_buf(),
            source,
        }
    }
}

/// The permissions a file gets when `write_atomic` creates it. A file that already exists keeps
/// its own, whatever this says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NewFileMode {
    /// Readable and writable by the person only (0600): for files that carry names, paths,
    /// messages or secrets.
    OwnerOnly,
    /// The system default, the process's umask applied to 0666.
    Umask,
    /// Exactly these permission bits (for example `0o644`), whatever the umask. Ignored off Unix.
    Mode(u32),
}

/// Write `body` as the file at `path`; the directory must exist.
///
/// The bytes go to a uniquely named staging file in the same directory (`.<name>.<pid>.<n>.tmp`,
/// created exclusively), are flushed to disk with `sync_all`, renamed over `path`, and the
/// directory is synced so the rename itself survives a power cut. A crash leaves at worst a stray
/// staging file beside an intact `path`; a failure removes its own.
///
/// A symlink at `path` is followed and its target replaced, so a dotfile-managed link stays a
/// link. An existing file's permissions are kept; a new file gets `new_file`. Writers of the same
/// file in this process take turns, so their staging files and renames never interleave.
///
/// # Errors
///
/// Returns [`WriteError`] when the target cannot be resolved or the staging file cannot be written,
/// synced or renamed into place.
pub fn write_atomic(path: &Path, body: &[u8], new_file: NewFileMode) -> Result<(), WriteError> {
    let target = link::resolve(path).map_err(|source| WriteError::at(path, source))?;
    lock::with(&target, || {
        publish(&target, new_file, |file| file.write_all(body))
    })
    .map_err(|source| WriteError::at(&target, source))
}

/// Write `body` as a new file at `path`, failing with [`io::ErrorKind::AlreadyExists`] (in the
/// error's `source`) when anything is there, a dangling symlink included, instead of replacing it.
///
/// The bytes are staged and synced as in [`write_atomic`], then hard-linked into place, so the
/// name is taken or not in one step and a reader never sees a partial file. The new file gets
/// `new_file`'s permissions. A symlink at `path` is not followed.
///
/// # Errors
///
/// Returns [`WriteError`] when `path` is taken, or the staging file cannot be written, synced or
/// linked into place (a file system without hard links cannot be written this way).
pub fn write_new(path: &Path, body: &[u8], new_file: NewFileMode) -> Result<(), WriteError> {
    lock::with(path, || {
        let staged = stage(path, new_file, false, |file| file.write_all(body))?;
        let linked = std::fs::hard_link(&staged, path);
        // Best effort: the staging name is only clutter once the file has its real name.
        let _ = std::fs::remove_file(&staged);
        linked?;
        sync_dir(directory_of(path))
    })
    .map_err(|source| WriteError::at(path, source))
}

/// The first half of a write, for a caller that already holds the path's lock and has resolved
/// any symlink: `fill` writes the bytes into a new staging file beside `target`, which is then
/// given the permissions `target` has now when `keep_mode` (else, and for a new file, `new_file`'s),
/// synced, and returned.
/// Until [`commit`] the target is untouched. A failure removes the staging file.
pub(crate) fn stage(
    target: &Path,
    new_file: NewFileMode,
    keep_mode: bool,
    fill: impl FnOnce(&mut File) -> io::Result<()>,
) -> io::Result<PathBuf> {
    if target.file_name().is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the path has no file name",
        ));
    }
    let kept = std::fs::metadata(target)
        .ok()
        .filter(|_| keep_mode)
        .filter(std::fs::Metadata::is_file)
        .map(|meta| meta.permissions());
    let (staged, mut staging_file) = create_staging(target, new_file)?;
    let written = fill(&mut staging_file)
        .and_then(|()| match kept {
            Some(permissions) => staging_file.set_permissions(permissions),
            None => Ok(()),
        })
        .and_then(|()| sync_file(&staging_file));
    drop(staging_file);
    match written {
        Ok(()) => Ok(staged),
        Err(error) => {
            // Best effort: the original is untouched either way, and a leftover is only clutter.
            let _ = std::fs::remove_file(&staged);
            Err(error)
        }
    }
}

/// The second half: the staged file replaces `target` and the directory entry is made durable.
/// The staged file is removed if the rename fails.
pub(crate) fn commit(staged: &Path, target: &Path) -> io::Result<()> {
    let moved = std::fs::rename(staged, target);
    if moved.is_err() {
        let _ = std::fs::remove_file(staged);
    }
    moved?;
    sync_dir(directory_of(target))
}

/// [`stage`] and [`commit`] in one, for a caller that already holds the path's lock.
pub(crate) fn publish(
    target: &Path,
    new_file: NewFileMode,
    fill: impl FnOnce(&mut File) -> io::Result<()>,
) -> io::Result<()> {
    let staged = stage(target, new_file, true, fill)?;
    commit(&staged, target)
}

/// A staging name beside `target` that no other write is using: `.<name>.<pid>.<n>.tmp`, a dot file
/// ending in `.tmp`, so no scan for the real files lists it.
///
/// The name is only proposed; the writer creates the file exclusively and asks again on a clash.
pub fn staging_beside(target: &Path) -> PathBuf {
    let number = STAGED.fetch_add(1, Ordering::Relaxed);
    let mut name = OsString::from(".");
    name.push(target.file_name().unwrap_or_default());
    name.push(format!(".{}.{number}.tmp", std::process::id()));
    directory_of(target).join(name)
}

/// The directory `target` is in (`.` for a bare file name).
pub(crate) fn directory_of(target: &Path) -> &Path {
    target
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn create_staging(target: &Path, new_file: NewFileMode) -> io::Result<(PathBuf, File)> {
    for _ in 0..TEMP_TRIES {
        let staged = staging_beside(target);
        match open_new(&staged, new_file) {
            Ok(file) => return Ok((staged, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "no free staging file name",
    ))
}

#[cfg_attr(not(unix), allow(unused_variables))]
fn open_new(path: &Path, new_file: NewFileMode) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    match new_file {
        NewFileMode::OwnerOnly => {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        NewFileMode::Mode(mode) => {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(mode);
        }
        NewFileMode::Umask => {}
    }
    let file = options.open(path)?;
    #[cfg(unix)]
    if let NewFileMode::Mode(mode) = new_file {
        use std::os::unix::fs::PermissionsExt;
        // The open applied the umask; the caller asked for these exact bits.
        file.set_permissions(std::fs::Permissions::from_mode(mode))?;
    }
    Ok(file)
}

fn sync_file(file: &File) -> io::Result<()> {
    #[cfg(test)]
    sync_count::file();
    file.sync_all()
}

/// Flush the directory entry of a rename to disk.
pub(crate) fn sync_dir(dir: &Path) -> io::Result<()> {
    #[cfg(test)]
    sync_count::dir();
    #[cfg(unix)]
    {
        File::open(dir)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        // Windows cannot open a directory as a file; the rename is as durable as it gets there.
        let _ = dir;
        Ok(())
    }
}

/// How many syncs this thread's writes made, so a test can see a write reach the disk.
#[cfg(test)]
pub(crate) mod sync_count {
    use std::cell::Cell;

    thread_local! {
        static FILES: Cell<usize> = const { Cell::new(0) };
        static DIRS: Cell<usize> = const { Cell::new(0) };
    }

    pub(super) fn file() {
        FILES.with(|n| n.set(n.get() + 1));
    }

    pub(super) fn dir() {
        DIRS.with(|n| n.set(n.get() + 1));
    }

    /// Files and directories synced on this thread so far.
    pub(crate) fn so_far() -> (usize, usize) {
        (FILES.with(Cell::get), DIRS.with(Cell::get))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_testkit::Scratch;

    #[cfg(unix)]
    fn bits(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path).expect("meta").permissions().mode() & 0o777
    }

    #[test]
    #[cfg(unix)]
    fn an_explicit_mode_is_exact_whatever_the_umask() {
        let scratch = Scratch::new("write");
        let path = scratch.join("a");
        write_atomic(&path, b"x", NewFileMode::Mode(0o640)).expect("write");
        assert_eq!(bits(&path), 0o640);
        let other = scratch.join("b");
        write_atomic(&other, b"x", NewFileMode::Mode(0o755)).expect("write");
        assert_eq!(bits(&other), 0o755);
    }

    #[test]
    fn write_new_creates_the_file_and_leaves_nothing_staged() {
        let scratch = Scratch::new("write");
        let path = scratch.join("shot.png");
        write_new(&path, b"png", NewFileMode::OwnerOnly).expect("write");
        assert_eq!(std::fs::read(&path).expect("read"), b"png");
        #[cfg(unix)]
        assert_eq!(bits(&path), 0o600);
        let count = std::fs::read_dir(scratch.path()).expect("dir").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn write_new_never_replaces_what_is_there() {
        let scratch = Scratch::new("write");
        let path = scratch.join("shot.png");
        std::fs::write(&path, b"mine").expect("seed");
        let error = write_new(&path, b"new", NewFileMode::Umask).expect_err("taken");
        assert_eq!(error.source.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&path).expect("read"), b"mine");
        let count = std::fs::read_dir(scratch.path()).expect("dir").count();
        assert_eq!(count, 1, "the staging file is gone");
    }

    #[test]
    #[cfg(unix)]
    fn write_new_does_not_follow_a_symlink() {
        let scratch = Scratch::new("write");
        let path = scratch.join("link.png");
        std::os::unix::fs::symlink(scratch.join("missing"), &path).expect("link");
        let error = write_new(&path, b"x", NewFileMode::Umask).expect_err("taken");
        assert_eq!(error.source.kind(), io::ErrorKind::AlreadyExists);
        assert!(!scratch.join("missing").exists());
    }
}
