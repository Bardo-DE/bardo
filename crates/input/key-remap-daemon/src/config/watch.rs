//! Live reload: a directory watch with `settings-store-fs`'s debounce, for the system file, the vendor file
//! and, through `spawn_reading`, for any other file in the directory (the keymap).
//!
//! The directory, not the file, is watched: editors and atomic writers replace the inode by
//! rename, which a file watch would lose.

use std::path::Path;
use std::sync::mpsc::Sender;

use key_remap_proto::Warning;
use settings_store_fs::watch_file_on_thread;

use super::load::{FILE_NAME, system};
use super::vendor::vendor;
use key_remap_core::ConfigLayer;

/// Watches `dir` on a thread of its own and sends every settled reload of the system layer to
/// `out`. Returns the watcher, which must be kept alive; dropping it ends the thread.
///
/// # Errors
///
/// Returns the `notify::Error` when `dir` cannot be watched.
pub fn spawn(
    dir: &Path,
    out: Sender<(ConfigLayer, Vec<Warning>)>,
) -> notify::Result<notify::RecommendedWatcher> {
    spawn_reading(dir, FILE_NAME, out, system)
}

/// Like `spawn` for the vendor file in `dir`, which must exist: a folder that is not there has
/// nothing to watch and is no layer.
pub fn spawn_vendor(
    dir: &Path,
    out: Sender<(ConfigLayer, Vec<Warning>)>,
) -> notify::Result<notify::RecommendedWatcher> {
    spawn_reading(dir, FILE_NAME, out, vendor)
}

/// Like `spawn` for the file `name` in `dir`: `read` is called on the directory after each
/// settled change and its result sent to `out`. The watch and its debounce are
/// `settings-store-fs`'s.
pub fn spawn_reading<T: Send + 'static>(
    dir: &Path,
    name: &'static str,
    out: Sender<T>,
    read: fn(&Path) -> T,
) -> notify::Result<notify::RecommendedWatcher> {
    let at = dir.to_path_buf();
    watch_file_on_thread(dir, name, format!("key-remap-watch-{name}"), move || {
        out.send(read(&at)).is_ok()
    })
}
