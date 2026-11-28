//! The person's own folders (Desktop, Documents, ...), as `user-dirs.dirs` names them.

use std::path::{Path, PathBuf};

/// The file the folders are named in, below `$XDG_CONFIG_HOME`.
pub const USER_DIRS_FILE: &str = "user-dirs.dirs";

/// The folders a `user-dirs.dirs` file names, by their variable (`XDG_DESKTOP_DIR`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserDirs(Vec<(String, PathBuf)>);

impl UserDirs {
    /// The folders `text` (the file's contents) names. A line is `NAME="value"` where the value is
    /// an absolute path, or `$HOME` followed by one; a comment, a line of any other shape, a
    /// relative path and a folder that is the home folder itself (how the file says "none") name
    /// nothing. A name given twice keeps its last value.
    #[must_use]
    pub fn parse(text: &str, home: &Path) -> UserDirs {
        let mut found: Vec<(String, PathBuf)> = Vec::new();
        for line in text.lines().map(str::trim) {
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            let name = name.trim();
            if name.starts_with('#') || name.is_empty() {
                continue;
            }
            let value = value.trim().trim_matches('"');
            let path = match value.strip_prefix("$HOME") {
                Some(rest) if rest.is_empty() || rest.starts_with('/') => {
                    home.join(rest.trim_start_matches('/'))
                }
                Some(_) => continue,
                None => PathBuf::from(value),
            };
            if !path.is_absolute() || path == home {
                continue;
            }
            found.retain(|(known, _)| known != name);
            found.push((name.to_owned(), path));
        }
        UserDirs(found)
    }

    /// The folder the variable `name` (`XDG_DOWNLOAD_DIR`) names, when the file names one.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Path> {
        self.0
            .iter()
            .find(|(known, _)| known == name)
            .map(|(_, path)| path.as_path())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn a_folder_is_home_relative_or_absolute_and_never_home_itself() {
        let text = "\
# a comment
XDG_DESKTOP_DIR=\"$HOME/Desktop\"
XDG_DOWNLOAD_DIR=\"/data/dl\"
XDG_MUSIC_DIR=\"$HOME/\"
XDG_VIDEOS_DIR=\"relative\"
XDG_PICTURES_DIR=\"$HOMEWORK/p\"
XDG_DOCUMENTS_DIR=\"$HOME/Old\"
XDG_DOCUMENTS_DIR=\"$HOME/Docs\"
not a line
";
        let dirs = UserDirs::parse(text, Path::new("/home/ada"));
        assert_eq!(
            dirs.get("XDG_DESKTOP_DIR"),
            Some(Path::new("/home/ada/Desktop"))
        );
        assert_eq!(dirs.get("XDG_DOWNLOAD_DIR"), Some(Path::new("/data/dl")));
        assert_eq!(
            dirs.get("XDG_DOCUMENTS_DIR"),
            Some(Path::new("/home/ada/Docs"))
        );
        for none in [
            "XDG_MUSIC_DIR",
            "XDG_VIDEOS_DIR",
            "XDG_PICTURES_DIR",
            "XDG_TEMPLATES_DIR",
        ] {
            assert_eq!(dirs.get(none), None, "{none}");
        }
    }
}
