//! Everything the edge reaches outside the process, chosen once by the binary: the person's
//! directories and the session bus. No implementation reads an
//! environment variable, a directory or a bus address itself; this file
//! is the one place the process is asked.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Where the person's files, caches and application entries live.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dirs {
    /// `$HOME`.
    pub home: PathBuf,
    /// `$XDG_CONFIG_HOME`, else `~/.config`.
    pub config: PathBuf,
    /// `$XDG_DATA_HOME`, else `~/.local/share`.
    pub data: PathBuf,
    /// `$XDG_CACHE_HOME`, else `~/.cache`.
    pub cache: PathBuf,
    /// `$XDG_STATE_HOME`, else `~/.local/state`: what must outlive a cache clean but is not a
    /// setting, such as the originals kept before a save in place.
    pub state: PathBuf,
    /// `$XDG_CONFIG_DIRS`, most important first.
    pub config_dirs: Vec<PathBuf>,
    /// `$XDG_DATA_DIRS`, most important first.
    pub data_dirs: Vec<PathBuf>,
}

impl Dirs {
    /// This process's, as the `dirs` crate and the XDG variables name them. Without a home at
    /// all everything hangs off `/`, where writes fail and are reported.
    #[must_use]
    pub fn from_process() -> Dirs {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        Dirs {
            config: dirs::config_dir().unwrap_or_else(|| home.join(".config")),
            data: dirs::data_dir().unwrap_or_else(|| home.join(".local/share")),
            cache: dirs::cache_dir().unwrap_or_else(|| home.join(".cache")),
            state: dirs::state_dir().unwrap_or_else(|| home.join(".local/state")),
            config_dirs: search_path("XDG_CONFIG_DIRS", SYSTEM_CONFIG),
            data_dirs: search_path("XDG_DATA_DIRS", SYSTEM_DATA),
            home,
        }
    }

    /// Every directory inside `root` (a test's scratch folder): `root/home`, `root/config`,
    /// `root/data`, `root/cache`, `root/state`, and one system directory each, `root/etc-xdg` and
    /// `root/share`.
    #[must_use]
    pub fn under(root: &Path) -> Dirs {
        Dirs {
            home: root.join("home"),
            config: root.join("config"),
            data: root.join("data"),
            cache: root.join("cache"),
            state: root.join("state"),
            config_dirs: vec![root.join("etc-xdg")],
            data_dirs: vec![root.join("share")],
        }
    }
}

/// The system directories searched when `XDG_CONFIG_DIRS` is unset. Windows has no such place.
#[cfg(not(windows))]
const SYSTEM_CONFIG: &str = "/etc/xdg";
#[cfg(windows)]
const SYSTEM_CONFIG: &str = "";

/// The system directories searched when `XDG_DATA_DIRS` is unset. Windows has no such place.
#[cfg(not(windows))]
const SYSTEM_DATA: &str = "/usr/local/share:/usr/share";
#[cfg(windows)]
const SYSTEM_DATA: &str = "";

/// A search path variable in the platform's own list syntax (colons, or semicolons on Windows), or
/// `default` when it is unset or empty.
fn search_path(variable: &str, default: &str) -> Vec<PathBuf> {
    let text = std::env::var_os(variable)
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| OsString::from(default));
    std::env::split_paths(&text)
        .filter(|part| !part.as_os_str().is_empty())
        .collect()
}

/// How the session bus is reached.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum BusRoute {
    /// The process's own: `DBUS_SESSION_BUS_ADDRESS`, else the standard socket.
    #[default]
    Usual,
    /// A bus by address: a private `dbus-daemon` (tests, dev scripts).
    Address(String),
    /// No bus at all: whatever needs one reports [`PlatformError::NoBus`].
    Absent,
}

/// The seams every implementation is built with: start from [`Env::from_process`] or
/// [`Env::isolated`] and change a seam with a `with_*` method.
#[derive(Debug, Clone)]
pub struct Env {
    /// The person's directories.
    pub dirs: Dirs,
    /// The session bus.
    pub session: BusRoute,
}

impl Env {
    /// The process's own directories, session bus.
    #[must_use]
    pub fn from_process() -> Env {
        Env {
            dirs: Dirs::from_process(),
            session: BusRoute::Usual,
        }
    }

    /// Nothing outside `scratch`: every directory inside it, and no bus. For tests that must
    /// never reach the person's session or files.
    #[must_use]
    pub fn isolated(scratch: &Path) -> Env {
        Env {
            dirs: Dirs::under(scratch),
            session: BusRoute::Absent,
        }
    }

    /// The same environment with the person's directories `dirs`.
    #[must_use]
    pub fn with_dirs(self, dirs: Dirs) -> Env {
        Env { dirs, ..self }
    }

    /// The same environment reaching the session bus by `session`.
    #[must_use]
    pub fn with_session(self, session: BusRoute) -> Env {
        Env { session, ..self }
    }
}
