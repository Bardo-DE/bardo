//! Where the daemon keeps its own files, from the XDG variables.

use base_xdg::Xdg;
use std::path::PathBuf;

/// The three places the update service reads and writes as a person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dirs {
    /// `$XDG_CONFIG_HOME/bardo/updater/config.toml`.
    pub config_file: PathBuf,
    /// `$XDG_STATE_HOME/bardo/updater/state.toml`.
    pub state_file: PathBuf,
    /// `$XDG_CACHE_HOME/bardo/updater`.
    pub cache_dir: PathBuf,
}

impl Dirs {
    /// Resolves the places from `xdg` (a test hands in its own, so none touches the real
    /// environment). `None` when there is neither the XDG variable nor `HOME`.
    pub fn resolve<F: Fn(&str) -> Option<String>>(xdg: &Xdg<F>) -> Option<Self> {
        Some(Self {
            config_file: xdg.config_home().ok()?.join("bardo/updater/config.toml"),
            state_file: xdg.state_home().ok()?.join("bardo/updater/state.toml"),
            cache_dir: xdg.cache_home().ok()?.join("bardo/updater"),
        })
    }
}
