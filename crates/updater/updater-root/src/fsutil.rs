//! Small durable file operations: symlink switch, directory modes and flush.

use std::fs::{self, DirBuilder, File};
use std::io;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path
        .file_name()
        .map_or_default(std::ffi::OsStr::to_os_string);
    name.push(suffix);
    path.with_file_name(name)
}

/// Flushes a directory's entries to disk.
///
/// # Errors
///
/// Returns the I/O error when the directory cannot be opened or flushed.
pub fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

/// Makes `path` and any missing parents, each created with `mode` (less the process's umask, so a
/// caller that needs the exact bits sets them afterwards).
///
/// # Errors
///
/// Returns the I/O error when a directory cannot be made.
pub fn create_dirs(path: &Path, mode: u32) -> io::Result<()> {
    DirBuilder::new().recursive(true).mode(mode).create(path)
}

/// Points the symlink `link` at `target` with one atomic rename: `link.new` is made first and
/// renamed over `link`.
///
/// # Errors
///
/// Returns the I/O error when the link cannot be made or renamed.
pub fn switch_link(link: &Path, target: &Path) -> io::Result<()> {
    let staged = sibling(link, ".new");
    match fs::remove_file(&staged) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    symlink(target, &staged)?;
    fs::rename(&staged, link)?;
    link.parent().map_or(Ok(()), sync_dir)
}

/// Removes a directory tree, treating "already gone" as done.
///
/// # Errors
///
/// Returns the I/O error when the tree cannot be removed, other than by being gone already.
pub fn remove_tree(dir: &Path) -> io::Result<()> {
    match fs::remove_dir_all(dir) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Gives `dir` and every directory under it the mode `0o755`, whatever the process's umask was when
/// they were made (the bundle's folders are plain, world-readable ones). Links are not followed.
///
/// # Errors
///
/// Returns the I/O error when a directory cannot be read or its mode set.
pub fn open_dirs(dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            open_dirs(&entry.path())?;
        }
    }
    fs::set_permissions(dir, fs::Permissions::from_mode(0o755))
}

/// Flushes every directory under `dir` (and `dir` itself).
///
/// # Errors
///
/// Returns the I/O error when a directory cannot be read or flushed.
pub fn sync_tree(dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            sync_tree(&entry.path())?;
        }
    }
    sync_dir(dir)
}
