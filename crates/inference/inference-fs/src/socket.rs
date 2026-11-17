//! The check of an engine's socket path before anything is spawned on it.

use inference_router::startup::{Cause, SUN_PATH};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::FileTypeExt;
use std::path::Path;

/// Looks at `path` before an engine is spawned on it.
///
/// A path too long to bind is refused, a socket left by an earlier run is removed, anything else
/// there is refused and kept (a symlink is not a socket, and is not followed).
///
/// # Errors
/// `Cause` when the path is too long to bind, is taken by anything but a socket, or the old socket
/// cannot be removed.
pub fn clear_socket(path: &Path) -> Result<(), Cause> {
    let len = path.as_os_str().as_bytes().len();
    if len >= SUN_PATH {
        return Err(Cause::SocketPathTooLong {
            path: path.to_path_buf(),
            len,
        });
    }
    let unusable = |kind| Cause::SocketPathUnusable {
        path: path.to_path_buf(),
        kind,
    };
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_socket() => {
            std::fs::remove_file(path).map_err(|e| unusable(e.kind()))
        }
        Ok(_) => Err(Cause::SocketPathTaken {
            path: path.to_path_buf(),
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(unusable(e.kind())),
    }
}

#[cfg(test)]
mod tests;
