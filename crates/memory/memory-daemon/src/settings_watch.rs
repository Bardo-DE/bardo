//! The settings, live (design/22 section 2): a directory watch on the folder that holds
//! `memory/settings.toml` (not on the file: the Settings app's atomic writer and every editor
//! replace the inode by rename), then the whole file is read again through the lenient reader
//! and the person's confirmations. The watch and its debounce are `settings-store-fs`'s.

use memory_host::Locator;
use memory_service::{Backend, MemoryService};
use memory_settings::{MemorySettings, lines};
use settings_store::Loaded;
use settings_store_fs::LocatedWatch;

/// Reads the file now over `base` and starts watching it. Must run inside a tokio runtime.
#[must_use]
pub fn watch_settings(locator: Locator, base: MemorySettings) -> LocatedWatch<MemorySettings> {
    let reader = locator.clone();
    let (watch, task) = LocatedWatch::start(locator.config(), move || reader.read(base));
    if let Some(task) = task {
        tokio::spawn(task);
    }
    watch
}

/// Logs what a read refused, then puts its values in force on `service`.
pub fn apply<B: Backend>(service: &MemoryService<B>, loaded: &Loaded<MemorySettings>) {
    for line in lines(loaded, "memoryd") {
        eprintln!("{line}");
    }
    service.apply_settings(loaded.value);
}

/// Waits for the next change of the file and applies it to `service`; `None` when the watch ended.
pub async fn apply_next<B: Backend>(
    service: &MemoryService<B>,
    settings: &mut LocatedWatch<MemorySettings>,
) -> Option<Loaded<MemorySettings>> {
    let loaded = settings.changed().await?;
    apply(service, &loaded);
    Some(loaded)
}
