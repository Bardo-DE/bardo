//! Where the settings file is: `$XDG_CONFIG_HOME/bardo/memory/settings.toml`, then
//! `bardo/memory/settings.toml` under each of `$XDG_CONFIG_DIRS` (design/22 section 2). The first file that reads wins.

use crate::files::CONFIG_CAP;
use base_fs::atomic::AtomicWrite;
use base_fs::read_text_capped;
use memory_core::WaitingSetting;
use memory_settings::{
    ConfirmError, Confirmations, MemorySettings, SETTINGS_FILE, gate, read, waiting,
};
use settings_store::{InvalidKey, Loaded};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The configuration directories, in order of precedence, and the store of the person's
/// confirmations (ALM-16) that gates what a read lets through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locator {
    config: settings_store_fs::Locator,
    confirmations: Option<PathBuf>,
}

impl Locator {
    /// The directories named by `XDG_CONFIG_HOME` (else `$HOME/.config`) and `XDG_CONFIG_DIRS`
    /// (else `/etc/xdg`).
    pub fn from_env(env: &impl Fn(&str) -> Option<String>) -> Self {
        Self {
            config: settings_store_fs::Locator::from_env(env, SETTINGS_FILE),
            confirmations: None,
        }
    }

    /// This locator with the confirmations kept in `file`. Without one, nothing is confirmed and a
    /// value looser than the shipped default never takes effect.
    #[must_use]
    pub fn confirming(mut self, file: PathBuf) -> Self {
        self.confirmations = Some(file);
        self
    }

    /// The `bardo` folders of the configuration directories, the person's first.
    #[must_use]
    pub fn dirs(&self) -> &[PathBuf] {
        self.config.dirs()
    }

    /// The directory a live watch looks at: the person's own `bardo/memory` directory, where the
    /// Settings app writes.
    #[must_use]
    pub fn watch_dir(&self) -> Option<PathBuf> {
        self.config.watch_dir()
    }

    /// The directory and file name a live watch looks at, for [`settings_store_fs::LocatedWatch`].
    #[must_use]
    pub fn config(&self) -> &settings_store_fs::Locator {
        &self.config
    }

    /// The confirmations as stored now; a store that is not there is nothing confirmed, a store
    /// that does not read is nothing confirmed and a report.
    fn stored(&self) -> (Confirmations, Option<InvalidKey>) {
        let Some(file) = &self.confirmations else {
            return (Confirmations::default(), None);
        };
        match read_text_capped(file, CONFIG_CAP) {
            Err(_) => (Confirmations::default(), None),
            Ok(text) => match Confirmations::parse(&text) {
                Ok(found) => (found, None),
                Err(e) => (
                    Confirmations::default(),
                    Some(InvalidKey {
                        path: file.display().to_string(),
                        reason: format!("not TOML: {e}"),
                    }),
                ),
            },
        }
    }

    /// The file as it is now over `base`, with every value looser than the shipped default that
    /// the person has not confirmed put back; no file is `base`, nothing to report.
    #[must_use]
    pub fn read(&self, base: MemorySettings) -> Loaded<MemorySettings> {
        let loaded = self.config.read(base, read);
        let (confirmed, unreadable) = self.stored();
        let mut gated = gate(loaded, &confirmed);
        gated.invalid.extend(unreadable);
        gated
    }

    /// The loose values the file holds now that the person has not confirmed, each with the
    /// shipped value in force, in key order: exactly the keys [`Locator::read`] puts back.
    #[must_use]
    pub fn waiting(&self) -> Vec<WaitingSetting> {
        waiting(&self.read(MemorySettings::default()))
    }

    /// Records that the person confirmed `values` (key to value): all of them are checked first,
    /// then the store is written atomically (a temp file, synced, renamed). The caller serialises
    /// calls. Refused when there is no store or a pair is not a confirmable setting.
    ///
    /// # Errors
    ///
    /// Returns a [`ConfirmError`] when there is no store, a pair is not a confirmable setting, or
    /// the store cannot be written.
    pub fn confirm(&self, values: &BTreeMap<String, String>) -> Result<(), ConfirmError> {
        let file = self
            .confirmations
            .as_ref()
            .ok_or_else(|| ConfirmError::Store("no store to keep confirmations in".to_owned()))?;
        // A store that does not read holds nothing, so it is written afresh: less is confirmed,
        // never more.
        let (have, _) = self.stored();
        let text = have.with(values)?.to_toml()?;
        AtomicWrite::PRIVATE
            .write(file, text.as_bytes())
            .map_err(|e| ConfirmError::Store(e.to_string()))
    }
}
