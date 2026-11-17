//! The person's computer-use settings (design/22 section 3.29): the `cua.*` keys the Settings app
//! writes to `$XDG_CONFIG_HOME/computer-use/settings.toml`, read leniently (a bad value falls back, per key,
//! and is logged) and found through the configuration directories. The pure part (`read`) is text
//! in, typed values out; the file is the edge (the caller's [`settings_store_fs::ReadText`]), and the live watch on it is the daemon's, which
//! hands each [`settings_store::Loaded`] to `Supervisor::adopt`.

mod apps;
mod keys;
#[cfg(test)]
mod locate_tests;
mod read;
#[cfg(test)]
mod tests;

use computer_use_run::ComputerUseConfig;

pub use apps::{AppAllow, AppRule, Apps, is_browser};
pub use read::read;

/// The schema computer use ships for its settings (design/22 section 9.2).
pub const SCHEMA: &str = include_str!("../../../dist/settings/computer-use.settings.toml");

/// The file the settings live in, under the configuration directory.
pub const SETTINGS_FILE: &str = "computer-use/settings.toml";

/// Where the settings file is.
///
/// `$XDG_CONFIG_HOME/bardo/computer-use/settings.toml`, then the same under each of
/// `$XDG_CONFIG_DIRS`. Read it with
/// [`Locator::read_with`](settings_store_fs::Locator::read_with)`(reader, base, read)`.
pub fn locator(env: &impl Fn(&str) -> Option<String>) -> settings_store_fs::Locator {
    settings_store_fs::Locator::from_env(env, SETTINGS_FILE)
}

/// Every value of the file, typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputerUseSettings {
    /// The proposed values, with the person's choices in them.
    pub config: ComputerUseConfig,
    /// `computer_use.apps`: the apps computer use may drive.
    pub apps: Apps,
}

impl ComputerUseSettings {
    /// The settings of a machine with no file: `config`'s values and no restriction on apps (what
    /// a supervisor built in code runs under; the daemon reads the file and lists the apps).
    #[must_use]
    pub fn over(config: ComputerUseConfig) -> Self {
        Self {
            config,
            apps: Apps::Unrestricted,
        }
    }
}
