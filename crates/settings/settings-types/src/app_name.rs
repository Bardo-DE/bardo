//! Which program a settings folder belongs to: [`AppName`] names the program's directory inside
//! an XDG base (config, state, data or cache).

use std::path::{Path, PathBuf};

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
    #[must_use]
    pub const fn desktop(name: &'static str) -> AppName {
        AppName {
            name,
            owner: Owner::Desktop,
        }
    }

    /// A program that is not part of the desktop: its folders are `<base>/<name>`.
    #[must_use]
    pub const fn foreign(name: &'static str) -> AppName {
        AppName {
            name,
            owner: Owner::Foreign,
        }
    }

    /// The program's own name (`bardo` for the desktop-wide one).
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// This program's folder inside `base`, one of the XDG bases (config, state, data, cache).
    #[must_use]
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
/// `shell/settings.toml`) inside `base`.
///
/// It is the same place [`AppName::under`] gives, for a name only known at run time. `bardo` is the
/// desktop-wide level; a program of [`FOREIGN_PROGRAMS`] sits directly in `base`; every other one
/// is a program of the desktop.
#[must_use]
pub fn app_folder(base: &Path, app: &str) -> PathBuf {
    if app == DESKTOP_LEVEL {
        base.join(DESKTOP_LEVEL)
    } else if FOREIGN_PROGRAMS.contains(&app) {
        base.join(app)
    } else {
        base.join(DESKTOP_LEVEL).join(app)
    }
}

#[cfg(test)]
mod tests {
    use super::{AppName, app_folder};
    use std::path::Path;

    #[test]
    fn a_program_of_the_desktop_sits_under_the_bardo_level() {
        assert_eq!(
            AppName::desktop("shell").under(Path::new("/xdg")),
            Path::new("/xdg/bardo/shell")
        );
        assert_eq!(
            AppName::MAILO.under(Path::new("/xdg")),
            Path::new("/xdg/mailo")
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
}
