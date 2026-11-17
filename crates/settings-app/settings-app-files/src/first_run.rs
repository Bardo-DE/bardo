//! Whether `bardo-settings --first-run` has already run: a one-line file in the Settings app's own state
//! directory, not a settings row (a settings row would be a key some schema must own, and the
//! first run is not a preference).

use base_fs::{NewFileMode, write_atomic};
use std::io;
use std::path::PathBuf;

/// The file's name under `<state home>/bardo/settings/`.
const FILE: &str = "first-run";

/// What the file holds: only that it exists matters.
const CONTENT: &str = "done\n";

/// `$XDG_STATE_HOME` (or a scratch directory standing in for it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateHome(pub PathBuf);

/// Whether the first run has happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstRunMark {
    path: PathBuf,
}

impl FirstRunMark {
    /// The mark under `home`: `<home>/bardo/settings/first-run`.
    #[must_use]
    pub fn under(home: &StateHome) -> FirstRunMark {
        FirstRunMark {
            path: settings_types::app_folder(&home.0, "settings").join(FILE),
        }
    }

    /// Whether the first run has happened: `--first-run` exits at once when it has, and opens the
    /// window when it has not.
    #[must_use]
    pub fn done(&self) -> bool {
        self.path.is_file()
    }

    /// Records that the first run happened.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the folder or the marker file cannot be created.
    pub fn record(&self) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        write_atomic(&self.path, CONTENT.as_bytes(), NewFileMode::Umask)
            .map_err(|error| io::Error::new(error.source.kind(), error))
    }
}
