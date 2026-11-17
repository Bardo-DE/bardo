//! The book in a file of the app's state folder.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use base_fs::{NewFileMode, ReadCap, read_text_capped, write_atomic};
use files_core::{FolderPath, FolderSettings};

use super::{FolderSettingsStore, PrefBook};
use crate::error::FolderSettingsError;

/// The most the file may hold: far above [`KEPT`](super::KEPT) folders.
const CAP: ReadCap = ReadCap::mib(1);

/// What has been made of the file so far.
#[derive(Debug)]
enum State {
    /// Not read yet.
    Unread,
    /// Read (or absent): the book to add to.
    Ready(PrefBook),
    /// There but unusable: it is left as it is and nothing is written over it.
    Unusable,
}

/// A [`FolderSettingsStore`] over one TOML file. The file is read once, on first use; a missing file is an
/// empty book, and one that is there but can't be read or understood is never written over.
#[derive(Debug)]
pub struct TomlFolderSettings {
    file: PathBuf,
    state: Mutex<State>,
}

impl TomlFolderSettings {
    /// A store over `file` (its folder is made on the first save). The caller says where: this
    /// crate reads no environment.
    #[must_use]
    pub fn new(file: PathBuf) -> TomlFolderSettings {
        TomlFolderSettings {
            file,
            state: Mutex::new(State::Unread),
        }
    }

    /// The state, read from the file if it has not been.
    fn state(&self) -> MutexGuard<'_, State> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if matches!(*state, State::Unread) {
            *state = match read_text_capped(&self.file, CAP) {
                Ok(text) => PrefBook::parse(&text).map_or(State::Unusable, State::Ready),
                Err(missing) if missing.is_missing() => State::Ready(PrefBook::default()),
                Err(_) => State::Unusable,
            };
        }
        state
    }
}

impl FolderSettingsStore for TomlFolderSettings {
    fn load(&self, folder: &FolderPath) -> FolderSettings {
        match &*self.state() {
            State::Ready(book) => book.get(folder).unwrap_or_default(),
            State::Unread | State::Unusable => FolderSettings::default(),
        }
    }

    fn save(&self, folder: &FolderPath, prefs: FolderSettings) -> Result<(), FolderSettingsError> {
        let mut state = self.state();
        let State::Ready(book) = &mut *state else {
            return Err(FolderSettingsError::Unusable);
        };
        book.set(folder.clone(), prefs);
        if let Some(dir) = self.file.parent() {
            std::fs::create_dir_all(dir).map_err(FolderSettingsError::Folder)?;
        }
        write_atomic(&self.file, book.render().as_bytes(), NewFileMode::OwnerOnly)
            .map_err(FolderSettingsError::Write)
    }
}
