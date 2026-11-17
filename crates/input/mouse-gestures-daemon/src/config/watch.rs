//! Live reload: a directory watch with `settings-store-fs`'s debounce (22-SETTINGS §2).
//!
//! The directory, not the file, is watched: editors and the Settings app's atomic writer both
//! replace the inode by rename, which a file watch would lose.

use super::load::{FILE_NAME, Loaded, load};
use settings_store_fs::watch_file_on_thread;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

/// Watches `dir` on a thread of its own and sends every settled reload to `out`. Returns the
/// watcher, which must be kept alive; dropping it ends the thread.
///
/// # Errors
/// `notify::Error` when `dir` cannot be created or watched.
pub fn spawn(dir: PathBuf, out: Sender<Loaded>) -> notify::Result<notify::RecommendedWatcher> {
    std::fs::create_dir_all(&dir).map_err(notify::Error::io)?;
    let at = dir.clone();
    watch_file_on_thread(&dir, FILE_NAME, "palmrest-config".to_owned(), move || {
        out.send(load(Some(at.as_path()))).is_ok()
    })
}
