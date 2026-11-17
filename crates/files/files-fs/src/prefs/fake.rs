//! A prefs store in memory.

use std::sync::{Mutex, MutexGuard, PoisonError};

use files_core::{FolderPath, FolderSettings};

use super::{FolderSettingsStore, PrefBook};
use crate::error::FolderSettingsError;

/// A [`FolderSettingsStore`] that keeps its [`PrefBook`] in memory; share one between two windows to
/// stand for a relaunch.
#[derive(Debug, Default)]
pub struct FakeFolderSettings {
    book: Mutex<PrefBook>,
}

impl FakeFolderSettings {
    /// A store that remembers nothing yet.
    #[must_use]
    pub fn new() -> FakeFolderSettings {
        FakeFolderSettings::default()
    }

    fn book(&self) -> MutexGuard<'_, PrefBook> {
        self.book.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl FolderSettingsStore for FakeFolderSettings {
    fn load(&self, folder: &FolderPath) -> FolderSettings {
        self.book().get(folder).unwrap_or_default()
    }

    fn save(&self, folder: &FolderPath, prefs: FolderSettings) -> Result<(), FolderSettingsError> {
        self.book().set(folder.clone(), prefs);
        Ok(())
    }
}
