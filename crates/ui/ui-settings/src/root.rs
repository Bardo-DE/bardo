//! Where a program's settings live: [`ConfigRoot`] says which directory tree, [`AppName`] which
//! program's directory inside it.
//!
//! Config, not data: a cosmetic choice is never a write to the file that holds someone's mail.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::xdg;

/// Which tree of the config, state and data bases a program's folder sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Owner {
    /// A program of the desktop: `<base>/bardo/<name>`.
    Desktop,
    /// The desktop as a whole (its look, its extensions): the `bardo` level itself, `<base>/bardo`.
    DesktopWide,
    /// A program that is not part of the desktop: `<base>/<name>`.
    Foreign,
}

/// The directory a program's files live under: `bardo/shell` for a program of the desktop,
/// `bardo` for the desktop as a whole, `mailo` for a foreign program.
///
/// Settings file ids keep the grammar `<app>/<file>` whichever owner the program has: the
/// owner only decides where the folder sits inside each base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AppName {
    name: &'static str,
    owner: Owner,
}

impl AppName {
    /// The desktop-wide settings (`appearance.toml`, `style.css`, `extensions.toml`), which sit
    /// directly in the `bardo` level.
    pub const QUIRE: AppName = AppName {
        name: "bardo",
        owner: Owner::DesktopWide,
    };
    /// mailo, a program outside the desktop: its folders stay at `<base>/mailo`.
    pub const MAILO: AppName = AppName::foreign("mailo");

    /// A program of the desktop (`shell`, `settings`, `files`): its folders are
    /// `<base>/bardo/<name>`.
    pub const fn desktop(name: &'static str) -> AppName {
        AppName {
            name,
            owner: Owner::Desktop,
        }
    }

    /// A program that is not part of the desktop: its folders are `<base>/<name>`.
    pub const fn foreign(name: &'static str) -> AppName {
        AppName {
            name,
            owner: Owner::Foreign,
        }
    }

    /// The program's own name (`bardo` for the desktop-wide one).
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// This program's folder inside `base`, one of the XDG bases (config, state, data, cache).
    pub fn under(self, base: &Path) -> PathBuf {
        match self.owner {
            Owner::Desktop => base.join(DESKTOP_LEVEL).join(self.name),
            Owner::DesktopWide => base.join(DESKTOP_LEVEL),
            Owner::Foreign => base.join(self.name),
        }
    }
}

/// The folder every program of the desktop shares inside each XDG base.
const DESKTOP_LEVEL: &str = "bardo";

/// The programs outside the desktop whose settings the desktop knows by their file id
/// (`mailo/settings.toml`): their folders stay at `<base>/<name>`.
const FOREIGN_PROGRAMS: &[&str] = &["mailo"];

/// The folder of the program named by the first part of a settings file id (`shell` of
/// `shell/settings.toml`) inside `base`: the same place [`AppName::under`] gives, for a name only
/// known at run time. `bardo` is the desktop-wide level; a program of [`FOREIGN_PROGRAMS`] sits
/// directly in `base`; every other one is a program of the desktop.
pub fn app_folder(base: &Path, app: &str) -> PathBuf {
    if app == DESKTOP_LEVEL {
        base.join(DESKTOP_LEVEL)
    } else if FOREIGN_PROGRAMS.contains(&app) {
        base.join(app)
    } else {
        base.join(DESKTOP_LEVEL).join(app)
    }
}

/// The tree every program's config directory sits in.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConfigRoot {
    /// The person's own: `$XDG_CONFIG_HOME`, else `$HOME/.config`, by the XDG rules in
    /// `crate::xdg` (an empty or relative value is ignored). The one place this crate reads
    /// the environment for the config directory.
    Xdg,
    /// A directory standing in for `$XDG_CONFIG_HOME`: a test's scratch directory, a dev
    /// script's private home.
    Scratch(PathBuf),
}

impl ConfigRoot {
    /// `app`'s config directory in this tree, or `None` when [`ConfigRoot::Xdg`] has neither
    /// variable to start from.
    pub fn dir(&self, app: AppName) -> Option<PathBuf> {
        match self {
            ConfigRoot::Xdg => xdg_dir(
                app,
                std::env::var_os("XDG_CONFIG_HOME").as_deref(),
                std::env::var_os("HOME").as_deref(),
            ),
            ConfigRoot::Scratch(base) => Some(app.under(base)),
        }
    }
}

impl ConfigRoot {
    /// `app`'s state directory in this tree: what changes as the program is used and is not a
    /// choice (`$XDG_STATE_HOME`, else `$HOME/.local/state`). A scratch root keeps it beside the
    /// config directory, in `.state`.
    pub fn state_dir(&self, app: AppName) -> Option<PathBuf> {
        match self {
            ConfigRoot::Xdg => xdg::base_dir(
                std::env::var_os("XDG_STATE_HOME").as_deref(),
                std::env::var_os("HOME").as_deref(),
                xdg::STATE_HOME_UNDER_HOME,
            )
            .map(|base| app.under(&base)),
            ConfigRoot::Scratch(base) => Some(app.under(&base.join(".state"))),
        }
    }
}

fn xdg_dir(app: AppName, xdg: Option<&OsStr>, home: Option<&OsStr>) -> Option<PathBuf> {
    xdg::base_dir(xdg, home, xdg::CONFIG_HOME_UNDER_HOME).map(|base| app.under(&base))
}

#[cfg(test)]
mod tests {
    use super::{AppName, ConfigRoot, app_folder, xdg_dir};
    use std::ffi::OsStr;
    use std::path::{Path, PathBuf};

    struct Case {
        name: &'static str,
        xdg: Option<&'static str>,
        home: Option<&'static str>,
        expect: Option<&'static str>,
    }

    const CASES: &[Case] = &[
        Case {
            name: "xdg wins",
            xdg: Some("/xdg"),
            home: Some("/home/ada"),
            expect: Some("/xdg/bardo"),
        },
        Case {
            name: "home when xdg is unset",
            xdg: None,
            home: Some("/home/ada"),
            expect: Some("/home/ada/.config/bardo"),
        },
        Case {
            name: "home when xdg is empty",
            xdg: Some(""),
            home: Some("/home/ada"),
            expect: Some("/home/ada/.config/bardo"),
        },
        Case {
            name: "home when xdg is relative",
            xdg: Some("config"),
            home: Some("/home/ada"),
            expect: Some("/home/ada/.config/bardo"),
        },
        Case {
            name: "neither",
            xdg: None,
            home: None,
            expect: None,
        },
        Case {
            name: "empty xdg and empty home is not the working directory",
            xdg: Some(""),
            home: Some(""),
            expect: None,
        },
        Case {
            name: "relative home is not a home",
            xdg: None,
            home: Some("ada"),
            expect: None,
        },
    ];

    #[test]
    fn the_config_directory_follows_xdg() {
        for case in CASES {
            assert_eq!(
                xdg_dir(
                    AppName::QUIRE,
                    case.xdg.map(OsStr::new),
                    case.home.map(OsStr::new)
                ),
                case.expect.map(PathBuf::from),
                "{}",
                case.name
            );
        }
    }

    #[test]
    fn the_app_name_is_the_last_component() {
        assert_eq!(
            xdg_dir(AppName::MAILO, Some(OsStr::new("/xdg")), None),
            Some(PathBuf::from("/xdg/mailo"))
        );
    }

    #[test]
    fn a_program_of_the_desktop_sits_under_the_bardo_level() {
        assert_eq!(
            xdg_dir(AppName::desktop("shell"), Some(OsStr::new("/xdg")), None),
            Some(PathBuf::from("/xdg/bardo/shell"))
        );
    }

    #[test]
    fn a_folder_named_at_run_time_is_where_the_typed_name_puts_it() {
        let base = Path::new("/xdg");
        assert_eq!(app_folder(base, "bardo"), AppName::QUIRE.under(base));
        assert_eq!(
            app_folder(base, "shell"),
            AppName::desktop("shell").under(base)
        );
        assert_eq!(app_folder(base, "mailo"), AppName::MAILO.under(base));
    }

    #[test]
    fn a_scratch_state_directory_follows_the_same_layout() {
        let root = ConfigRoot::Scratch(PathBuf::from("/scratch"));
        assert_eq!(
            root.state_dir(AppName::desktop("files")),
            Some(PathBuf::from("/scratch/.state/bardo/files"))
        );
        assert_eq!(
            root.state_dir(AppName::MAILO),
            Some(PathBuf::from("/scratch/.state/mailo"))
        );
    }

    #[test]
    fn a_scratch_root_stands_in_for_xdg_config_home() {
        let root = ConfigRoot::Scratch(PathBuf::from("/scratch"));
        assert_eq!(
            root.dir(AppName::QUIRE),
            Some(PathBuf::from("/scratch/bardo"))
        );
    }
}
