//! How each folder was last shown, remembered between runs.
//!
//! [`FolderSettingsStore`] is the seam: [`TomlFolderSettings`] keeps the book in one file in the app's state folder,
//! `FakeFolderSettings` (feature `testing`) keeps it in memory. The book itself ([`PrefBook`]) is pure data
//! and text, so what it keeps and what it forgets is tested without a disk.

mod book;
#[cfg(any(test, feature = "testing"))]
mod fake;
mod toml_store;

use files_core::{FolderPath, FolderSettings};

use crate::error::FolderSettingsError;

pub use book::{KEPT, PrefBook};
#[cfg(any(test, feature = "testing"))]
pub use fake::FakeFolderSettings;
pub use toml_store::TomlFolderSettings;

/// A store that remembers nothing: every folder opens with the defaults. What a window has until
/// it is given a real one.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoFolderSettings;

impl FolderSettingsStore for NoFolderSettings {
    fn load(&self, _folder: &FolderPath) -> FolderSettings {
        FolderSettings::default()
    }

    fn save(
        &self,
        _folder: &FolderPath,
        _prefs: FolderSettings,
    ) -> Result<(), FolderSettingsError> {
        Ok(())
    }
}

/// Where each folder's view and sort are kept. Real: [`TomlFolderSettings`]; for tests: `FakeFolderSettings`
/// (feature `testing`).
pub trait FolderSettingsStore: Send + Sync {
    /// How `folder` was last shown; the defaults for a folder never changed (or when the store
    /// can't be read).
    fn load(&self, folder: &FolderPath) -> FolderSettings;

    /// Remembers that `folder` is shown with `prefs`.
    ///
    /// # Errors
    /// `FolderSettingsError` when the preferences cannot be written.
    fn save(&self, folder: &FolderPath, prefs: FolderSettings) -> Result<(), FolderSettingsError>;
}
