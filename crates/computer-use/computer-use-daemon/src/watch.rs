//! The settings, live (design/22 section 2): a directory watch on the folder that holds
//! `computer-use/settings.toml` (not on the file: the Settings app's atomic writer and every
//! editor replace the inode by rename), then the whole file is read again through the lenient
//! reader. The watch and its debounce are `settings-store-fs`'s.

use computer_use_host::{ComputerUseSettings, read};
use settings_store_fs::{LocatedWatch, Locator};

/// Reads the file now over `base` and starts watching it. Must run inside a tokio runtime.
#[must_use]
pub fn watch_settings(
    locator: &Locator,
    base: ComputerUseSettings,
) -> LocatedWatch<ComputerUseSettings> {
    let reader = locator.clone();
    let (watch, task) = LocatedWatch::start(locator, move || reader.read(base.clone(), read));
    if let Some(task) = task {
        tokio::spawn(task);
    }
    watch
}
