//! Where the settings file is: `$XDG_CONFIG_HOME/bardo/actions/settings.toml`, then the
//! same under each of `$XDG_CONFIG_DIRS` (design/22 section 2). The first file that reads wins.

use crate::read::{Loaded, read};
use crate::{AgentSettings, SETTINGS_FILE};
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

    /// The `bardo` folders of the configuration directories, the person's first.
    #[must_use]
    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }

    /// The directory a live watch looks at: the person's own `bardo/actions` directory, where the
    /// Settings app writes.
    #[must_use]
    pub fn watch_dir(&self) -> Option<PathBuf> {
        let person = self.dirs.first()?;
        Path::new(SETTINGS_FILE)
            .parent()
            .map(|name| person.join(name))
    }

    /// The first settings file that can be read, as text.
    fn text(&self) -> Option<String> {
        self.dirs
            .iter()
            .map(|d| d.join(SETTINGS_FILE))
            .find_map(|path| std::fs::read_to_string(path).ok())
    }

    /// The file as it is now over `base`; no file is `base`, nothing to report.
    #[must_use]
    pub fn read(&self, base: AgentSettings) -> Loaded {
        match self.text() {
            Some(text) => read(&text, base),
            None => Loaded::of(base),
        }
    }
}
