//! The folders desktop entries are searched in.

use base_xdg::Xdg;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The folder of desktop entries inside a data directory.
const APPLICATIONS: &str = "applications";
/// The system Flatpak install's exports.
const FLATPAK_SYSTEM: &str = "/var/lib/flatpak/exports/share/applications";
/// Snap's exports.
const SNAP: &str = "/var/lib/snapd/desktop/applications";

/// The three environment values that decide where entries are, as the caller read them
/// (`xdg_fs::process_xdg_env` reads this process's). Keeping them in a value makes the resolution
/// pure: a test hands in what it wants.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct XdgEnv {
    /// `XDG_DATA_HOME`.
    pub data_home: Option<OsString>,
    /// `XDG_DATA_DIRS`.
    pub data_dirs: Option<OsString>,
    /// `HOME`, for the default of `XDG_DATA_HOME`.
    pub home: Option<PathBuf>,
}

impl XdgEnv {
    /// The XDG rules over these three values: the folder a variable names when it is absolute,
    /// else the default under an absolute `HOME`.
    fn rules(&self) -> Xdg<impl Fn(&str) -> Option<String> + '_> {
        Xdg::new(|name| {
            let value = match name {
                "XDG_DATA_HOME" => self.data_home.as_deref(),
                "XDG_DATA_DIRS" => self.data_dirs.as_deref(),
                "HOME" => self.home.as_deref().map(Path::as_os_str),
                _ => None,
            };
            value.and_then(|value| value.to_str()).map(str::to_owned)
        })
    }

    /// The person's data folder: `XDG_DATA_HOME` when it is an absolute path, else
    /// `$HOME/.local/share`; none when neither is usable.
    #[must_use]
    pub fn data_home(&self) -> Option<PathBuf> {
        self.rules().data_home().ok()
    }

    /// The system data folders: `XDG_DATA_DIRS` without its empty and relative entries, or
    /// `/usr/local/share:/usr/share` when it is unset or empty.
    #[must_use]
    pub fn data_dirs(&self) -> Vec<PathBuf> {
        self.rules().data_dirs()
    }

    /// The `applications` folders, most important first: the person's, each system one, then the
    /// app stores' export folders (the user's Flatpak install, the system's, Snap's) that are not
    /// already listed. A session that has sourced the stores' profile scripts lists them itself,
    /// at the position it chose, and keeps it.
    pub fn applications_dirs(&self) -> Vec<PathBuf> {
        let home = self.data_home();
        let mut dirs: Vec<PathBuf> = home
            .iter()
            .map(PathBuf::as_path)
            .chain(self.data_dirs().iter().map(PathBuf::as_path))
            .map(|dir| dir.join(APPLICATIONS))
            .collect();
        for store in store_dirs(home.as_deref()) {
            if !dirs.contains(&store) {
                dirs.push(store);
            }
        }
        dirs
    }
}

/// The app stores' export folders, the person's own first.
fn store_dirs(data_home: Option<&Path>) -> Vec<PathBuf> {
    data_home
        .map(|home| home.join("flatpak/exports/share").join(APPLICATIONS))
        .into_iter()
        .chain([PathBuf::from(FLATPAK_SYSTEM), PathBuf::from(SNAP)])
        .collect()
}
