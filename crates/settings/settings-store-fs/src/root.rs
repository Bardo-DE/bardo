//! Where a program's settings live: [`ConfigRoot`] says which directory tree (the person's XDG
//! config home, or a scratch directory standing in for it); the program's own folder inside it is
//! [`AppName`].
//!
//! Config, not data: a cosmetic choice is never a write to the file that holds someone's mail.

use std::path::PathBuf;

use base_xdg::Xdg;
use settings_types::AppName;

/// The tree every program's config directory sits in.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConfigRoot {
    /// The person's own: `$XDG_CONFIG_HOME`, else `$HOME/.config`, by the XDG rules in
    /// `base_xdg` (an empty or relative value is ignored). The one place this crate reads
    /// the environment for the config directory.
    Xdg,
    /// A directory standing in for `$XDG_CONFIG_HOME`: a test's scratch directory, a dev
    /// script's private home.
    Scratch(PathBuf),
}

impl ConfigRoot {
    /// `app`'s config directory in this tree, or `None` when [`ConfigRoot::Xdg`] has neither
    /// variable to start from.
    #[must_use]
    pub fn dir(&self, app: AppName) -> Option<PathBuf> {
        match self {
            ConfigRoot::Xdg => Xdg::process()
                .config_home()
                .ok()
                .map(|base| app.under(&base)),
            ConfigRoot::Scratch(base) => Some(app.under(base)),
        }
    }
}

impl ConfigRoot {
    /// `app`'s state directory in this tree: what changes as the program is used and is not a
    /// choice (`$XDG_STATE_HOME`, else `$HOME/.local/state`). A scratch root keeps it beside the
    /// config directory, in `.state`.
    #[must_use]
    pub fn state_dir(&self, app: AppName) -> Option<PathBuf> {
        match self {
            ConfigRoot::Xdg => Xdg::process()
                .state_home()
                .ok()
                .map(|base| app.under(&base)),
            ConfigRoot::Scratch(base) => Some(app.under(&base.join(".state"))),
        }
    }
}

/// `app`'s config directory for the given `XDG_CONFIG_HOME` and `HOME` values, by the XDG rules.
#[cfg(test)]
fn xdg_dir(
    app: AppName,
    xdg: Option<&std::ffi::OsStr>,
    home: Option<&std::ffi::OsStr>,
) -> Option<PathBuf> {
    let text = |value: Option<&std::ffi::OsStr>| {
        value.and_then(std::ffi::OsStr::to_str).map(str::to_owned)
    };
    Xdg::new(|name: &str| match name {
        "XDG_CONFIG_HOME" => text(xdg),
        "HOME" => text(home),
        _ => None,
    })
    .config_home()
    .ok()
    .map(|base| app.under(&base))
}

#[cfg(test)]
mod tests {
    use super::{ConfigRoot, xdg_dir};
    use settings_types::AppName;
    use std::ffi::OsStr;
    use std::path::PathBuf;

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
                    AppName::BARDO,
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
            root.dir(AppName::BARDO),
            Some(PathBuf::from("/scratch/bardo"))
        );
    }
}
