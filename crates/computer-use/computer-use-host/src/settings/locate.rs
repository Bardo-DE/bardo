//! Where the settings file is: `$XDG_CONFIG_HOME/bardo/computer-use/settings.toml`, then the same
//! under each of `$XDG_CONFIG_DIRS` (design/22 section 2). The first file that reads wins.

use super::read::{Fallback, Loaded, Why, read};
use super::text::{ReadFailure, ReadText};
use super::{CuaSettings, SETTINGS_FILE};
use base_xdg::Xdg;
use std::path::{Path, PathBuf};

/// The configuration directories, in order of precedence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locator {
    dirs: Vec<PathBuf>,
}

impl Locator {
    /// The directories named by `XDG_CONFIG_HOME` (else `$HOME/.config`) and `XDG_CONFIG_DIRS`
    /// (else `/etc/xdg`).
    pub fn from_env(env: &impl Fn(&str) -> Option<String>) -> Self {
        let xdg = Xdg::new(env);
        Self {
            dirs: xdg
                .config_home()
                .ok()
                .into_iter()
                .chain(xdg.config_dirs())
                .map(|dir| dir.join("bardo"))
                .collect(),
        }
    }

    /// The directory a live watch looks at: the person's own `bardo/computer-use` directory, where the Settings
    /// app writes.
    #[must_use]
    pub fn watch_dir(&self) -> Option<PathBuf> {
        let person = self.dirs.first()?;
        Path::new(SETTINGS_FILE)
            .parent()
            .map(|name| person.join(name))
    }

    /// The file as `reader` shows it over `base`: the first one that reads; no file is `base`,
    /// nothing to report.
    pub fn read(&self, reader: &impl ReadText, base: &CuaSettings) -> Loaded {
        let mut unreadable = Vec::new();
        let mut text = None;
        for path in self.dirs.iter().map(|d| d.join(SETTINGS_FILE)) {
            match reader.read_text(&path) {
                Ok(found) => {
                    text = Some(found);
                    break;
                }
                // No file in this directory is the ordinary case.
                Err(ReadFailure::Missing) => {}
                // A file that is there and cannot be read is not the same as no file: it is
                // reported, so a sandbox that hides the settings does not fail silently.
                Err(ReadFailure::Unreadable { source }) => unreadable.push(Fallback {
                    key: path.display().to_string(),
                    why: Why::Unreadable(source.to_string()),
                }),
            }
        }
        let mut loaded = match text {
            Some(text) => read(&text, base),
            None => Loaded::of(base.clone()),
        };
        unreadable.append(&mut loaded.fallbacks);
        loaded.fallbacks = unreadable;
        loaded
    }
}
