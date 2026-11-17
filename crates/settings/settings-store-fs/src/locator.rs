//! Where a daemon's settings file is: `$XDG_CONFIG_HOME/bardo/<file>`, then the same under each of
//! `$XDG_CONFIG_DIRS` (design/22 section 2). The first file that reads wins whole; a file that is
//! there and cannot be read is reported and the next directory is tried.

use crate::text::{FileText, SETTINGS_CAP, file_text};
use base_xdg::Xdg;
use settings_store::{InvalidKey, Loaded};
use std::path::{Path, PathBuf};

/// Why a file gave no text.
#[derive(Debug, thiserror::Error)]
pub enum ReadFailure {
    /// There is no file. The ordinary case for a configuration directory.
    #[error("no such file")]
    Missing,
    /// The file is there and cannot be read.
    #[error("unreadable: {reason}")]
    Unreadable {
        /// Why, as a short phrase.
        reason: String,
    },
}

/// Reads the text of one file. A seam: a test shows the locator plain values, the daemon the disk.
pub trait ReadText {
    /// The text at `path`.
    ///
    /// # Errors
    ///
    /// [`ReadFailure::Missing`] when there is no such file, [`ReadFailure::Unreadable`] when there
    /// is one and it cannot be read.
    fn read_text(&self, path: &Path) -> Result<String, ReadFailure>;
}

/// The disk, through the one capped reader of settings files.
#[derive(Debug, Clone, Copy, Default)]
pub struct DiskText;

impl ReadText for DiskText {
    fn read_text(&self, path: &Path) -> Result<String, ReadFailure> {
        match file_text(path, SETTINGS_CAP) {
            FileText::Text(text) => Ok(text),
            FileText::Missing => Err(ReadFailure::Missing),
            FileText::Unusable(reason) => Err(ReadFailure::Unreadable { reason }),
        }
    }
}

/// The configuration directories, in order of precedence, and the settings file inside them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locator {
    dirs: Vec<PathBuf>,
    file: &'static str,
}

impl Locator {
    /// The `bardo` folders named by `XDG_CONFIG_HOME` (else `$HOME/.config`) and `XDG_CONFIG_DIRS`
    /// (else `/etc/xdg`), for the settings file `file` (`actions/settings.toml`).
    pub fn from_env(env: &impl Fn(&str) -> Option<String>, file: &'static str) -> Self {
        let xdg = Xdg::new(env);
        Self {
            dirs: xdg
                .config_home()
                .ok()
                .into_iter()
                .chain(xdg.config_dirs())
                .map(|dir| dir.join("bardo"))
                .collect(),
            file,
        }
    }

    /// The `bardo` folders of the configuration directories, the person's first.
    #[must_use]
    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }

    /// The directory a live watch looks at: the person's own folder of the file, where the
    /// Settings app writes.
    #[must_use]
    pub fn watch_dir(&self) -> Option<PathBuf> {
        let person = self.dirs.first()?;
        Path::new(self.file)
            .parent()
            .map(|folder| person.join(folder))
    }

    /// The name of the file a live watch looks for in [`watch_dir`](Locator::watch_dir).
    #[must_use]
    pub fn file_name(&self) -> Option<&std::ffi::OsStr> {
        Path::new(self.file).file_name()
    }

    /// The file as `reader` shows it, read by `parse` over `base`: the first one that reads wins
    /// whole. No file is `base` with nothing to report; a file that is there and cannot be read
    /// is reported in [`Loaded::invalid`] (so a sandbox that hides the settings does not fail
    /// silently) and the next directory is tried.
    pub fn read_with<T>(
        &self,
        reader: &impl ReadText,
        base: T,
        parse: impl FnOnce(&str, T) -> Loaded<T>,
    ) -> Loaded<T> {
        let mut unreadable = Vec::new();
        let mut text = None;
        for path in self.dirs.iter().map(|dir| dir.join(self.file)) {
            match reader.read_text(&path) {
                Ok(found) => {
                    text = Some(found);
                    break;
                }
                // No file in this directory is the ordinary case.
                Err(ReadFailure::Missing) => {}
                Err(ReadFailure::Unreadable { reason }) => unreadable.push(InvalidKey {
                    path: path.display().to_string(),
                    reason: format!("could not be read: {reason}"),
                }),
            }
        }
        let mut loaded = match text {
            Some(text) => parse(&text, base),
            None => Loaded::clean(base),
        };
        unreadable.append(&mut loaded.invalid);
        loaded.invalid = unreadable;
        loaded
    }

    /// [`read_with`](Locator::read_with) over the disk.
    pub fn read<T>(&self, base: T, parse: impl FnOnce(&str, T) -> Loaded<T>) -> Loaded<T> {
        self.read_with(&DiskText, base, parse)
    }
}
