//! The settings inferd serves and reads (design/22 section 5, the Intelligence page): the rows of
//! `dist/inference.settings.toml` and the `ai.model.<slot>.<tier>` picker, a live module.
//!
//! What the rows mean (the file's `[ai]` table, [`inference_settings::resolve`], the slugs and
//! paths) is the library `inference-settings`; what is here is the daemon's use of it:
//!
//! * [`Live`] holds the settings in force, shared by every clone of `Engines`; a session is routed
//!   by what is in force when it opens.
//! * [`ConfigFile`] reads and edits the file, and [`Reload`] puts what it says in force again: the
//!   daemon polls the file's mtime, `Rescan` reloads at once, and a `Set` of the module applies
//!   what it wrote.
//! * [`InferdSettings`] is `org.bardo.SettingsModule1` at `INFERENCE_SETTINGS_PATH`.

mod file;
mod local_engines;
mod missing;
mod module;
mod pool;
mod programs;

pub use file::{ConfigFile, FileError, Reload, ReloadFailed, Reloaded};
pub use local_engines::LocalEngines;
pub(crate) use missing::engine_name;
pub use module::{InferdSettings, serve_settings};
pub use pool::HelperPools;

use inference_settings::Settings;
use std::sync::Arc;
use tokio::sync::watch;

/// The settings in force: replaced whole, read as a snapshot, and followed by whoever
/// [`Live::subscribe`]s.
#[derive(Debug, Clone)]
pub struct Live(Arc<watch::Sender<Arc<Settings>>>);

impl Default for Live {
    fn default() -> Self {
        Self::new(Settings::default())
    }
}

impl Live {
    /// A holder of `settings`.
    #[must_use]
    pub fn new(settings: Settings) -> Self {
        Self(Arc::new(watch::Sender::new(Arc::new(settings))))
    }

    /// What is in force.
    #[must_use]
    pub fn get(&self) -> Arc<Settings> {
        Arc::clone(&self.0.borrow())
    }

    /// Puts `settings` in force.
    pub fn set(&self, settings: Settings) {
        self.0.send_replace(Arc::new(settings));
    }

    /// Follows the settings: the receiver changes whenever they are put in force again (also
    /// when nothing differs).
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<Arc<Settings>> {
        self.0.subscribe()
    }
}

#[cfg(test)]
mod tests;
