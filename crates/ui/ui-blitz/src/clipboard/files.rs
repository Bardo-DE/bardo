//! The system clipboard's file list, through arboard.

use std::path::PathBuf;

use ui_host_hooks::clipboard::{ClipboardError, ClipboardFiles, FileOp};

/// Offer `paths` on the system clipboard as `text/uri-list` (arboard's file list).
///
/// The `arboard::Clipboard` that wrote it stays alive for the process: on X11 it owns the
/// selection and answers every paste request. The next write replaces it.
pub(super) fn write_system(paths: &[PathBuf]) -> Result<(), ClipboardError> {
    use std::sync::{Mutex, OnceLock, PoisonError};
    static OWNER: OnceLock<Mutex<Option<arboard::Clipboard>>> = OnceLock::new();
    let mut owner = OWNER
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if owner.is_none() {
        *owner = Some(arboard::Clipboard::new().map_err(ClipboardError::refused)?);
    }
    owner
        .as_mut()
        .ok_or(ClipboardError::Unavailable)?
        .set()
        .file_list(paths)
        .map_err(ClipboardError::refused)
}

/// The system clipboard's `text/uri-list`, as the list's text (read on the capped worker).
pub(super) fn read_system() -> Option<String> {
    let paths = arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.get().file_list())
        .ok()?;
    Some(ClipboardFiles::new(FileOp::Copy, paths).to_uri_list())
}
