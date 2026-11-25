//! The person's settings, live (design/22 section 2): a directory watch on the folder that holds
//! `actions/settings.toml` (not on the file: the Settings app's atomic writer and every editor
//! replace the inode by rename), then the whole file is read again through the lenient reader.
//! The watch and its debounce are `settings-store-fs`'s. The router is handed the result; the
//! next call decides by it.

use action_router::{Router, Seams};
use action_settings::{AgentSettings, read};
use settings_store::Loaded;
use settings_store_fs::{LocatedWatch, Locator};

/// Reads the file now over `base` and starts watching it. Must run inside a tokio runtime.
#[must_use]
pub fn watch_settings(locator: &Locator, base: AgentSettings) -> LocatedWatch<AgentSettings> {
    let reader = locator.clone();
    let (watch, task) = LocatedWatch::start(locator, move || reader.read(base, read));
    if let Some(task) = task {
        tokio::spawn(task);
    }
    watch
}

/// Logs what a read refused, then puts its values in force on `router`.
pub fn apply<S: Seams>(router: &Router<S>, loaded: &Loaded<AgentSettings>) {
    for line in loaded.lines("actiond") {
        eprintln!("{line}");
    }
    router.apply_settings(loaded.value.agent);
}

/// Waits for the next change of the file and applies it to `router`; `None` when the watch ended.
pub async fn apply_next<S: Seams>(
    router: &Router<S>,
    settings: &mut LocatedWatch<AgentSettings>,
) -> Option<Loaded<AgentSettings>> {
    let loaded = settings.changed().await?;
    apply(router, &loaded);
    Some(loaded)
}
