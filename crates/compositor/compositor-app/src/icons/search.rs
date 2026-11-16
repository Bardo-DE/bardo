//! Where the desktop entries are.

use base_xdg::Xdg;
use std::path::PathBuf;

/// Where app icons are looked for: through the desktop entries in these folders, most important
/// first, and the installed icon themes.
///
/// (No icons are looked for or drawn, in the jail and the test kit, when there is no `IconSearch`.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconSearch {
    /// The `applications` folders, most important first.
    pub applications: Vec<PathBuf>,
    /// The config folder the desktop's icon theme is named under (see
    /// `compositor_icons::theme_file`); `hicolor` when none.
    pub config: Option<PathBuf>,
}

impl IconSearch {
    /// The XDG folders: `$XDG_DATA_HOME` (else `$HOME/.local/share`) first, then each of
    /// `$XDG_DATA_DIRS` (else `/usr/local/share:/usr/share`), each with `applications` added;
    /// the config folder is `$XDG_CONFIG_HOME` (else `$HOME/.config`).
    ///
    /// ```
    /// use base_xdg::Xdg;
    /// use compositor_app::icons::IconSearch;
    /// use std::path::PathBuf;
    ///
    /// let xdg = Xdg::new(|name| match name {
    ///     "HOME" => Some("/home/a".to_owned()),
    ///     "XDG_DATA_DIRS" => Some("/x:/y".to_owned()),
    ///     _ => None,
    /// });
    /// let want = ["/home/a/.local/share/applications", "/x/applications", "/y/applications"];
    /// assert_eq!(
    ///     IconSearch::xdg(&xdg),
    ///     IconSearch {
    ///         applications: want.map(PathBuf::from).to_vec(),
    ///         config: Some(PathBuf::from("/home/a/.config")),
    ///     }
    /// );
    /// ```
    pub fn xdg<F: Fn(&str) -> Option<String>>(xdg: &Xdg<F>) -> IconSearch {
        let applications = xdg
            .data_home()
            .into_iter()
            .chain(xdg.data_dirs())
            .map(|dir| dir.join("applications"))
            .collect();
        IconSearch {
            applications,
            config: xdg.config_home().ok(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn search(vars: &[(&str, &str)]) -> IconSearch {
        IconSearch::xdg(&Xdg::new(|name| {
            vars.iter()
                .find(|(var, _)| *var == name)
                .map(|(_, value)| (*value).to_owned())
        }))
    }

    #[test]
    fn the_folders_follow_the_xdg_defaults() {
        // name, variables, want
        let cases: [(&str, &[(&str, &str)], Vec<&str>); 4] = [
            (
                "both unset",
                &[("HOME", "/h")],
                vec![
                    "/h/.local/share/applications",
                    "/usr/local/share/applications",
                    "/usr/share/applications",
                ],
            ),
            (
                "a data home wins over the home folder",
                &[
                    ("XDG_DATA_HOME", "/d"),
                    ("HOME", "/h"),
                    ("XDG_DATA_DIRS", "/a"),
                ],
                vec!["/d/applications", "/a/applications"],
            ),
            (
                "a relative data home is ignored",
                &[
                    ("XDG_DATA_HOME", "d"),
                    ("HOME", "/h"),
                    ("XDG_DATA_DIRS", "/a::/b"),
                ],
                vec![
                    "/h/.local/share/applications",
                    "/a/applications",
                    "/b/applications",
                ],
            ),
            (
                "no home at all",
                &[("XDG_DATA_DIRS", "/a")],
                vec!["/a/applications"],
            ),
        ];
        for (name, vars, want) in cases {
            let want: Vec<PathBuf> = want.into_iter().map(PathBuf::from).collect();
            assert_eq!(search(vars).applications, want, "{name}");
        }
    }

    #[test]
    fn the_config_folder_follows_the_xdg_defaults() {
        // name, variables, want
        let cases: [(&str, &[(&str, &str)], Option<&str>); 4] = [
            (
                "set",
                &[("XDG_CONFIG_HOME", "/c"), ("HOME", "/h")],
                Some("/c"),
            ),
            ("unset", &[("HOME", "/h")], Some("/h/.config")),
            (
                "relative is ignored",
                &[("XDG_CONFIG_HOME", "c"), ("HOME", "/h")],
                Some("/h/.config"),
            ),
            ("nothing", &[], None),
        ];
        for (name, vars, want) in cases {
            assert_eq!(search(vars).config, want.map(PathBuf::from), "{name}");
        }
    }
}
