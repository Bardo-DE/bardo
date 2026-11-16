//! Writing a state file so that a crash, a full disk or a power cut leaves the old contents or the
//! new, never half of either: the one writer is `base_fs::write_atomic`, and what this module adds
//! is the folder made first and the file named in the error. A new file is private to its owner,
//! a file that exists keeps its permissions, and a symbolic link in the file's place is written
//! through.
//!
//! A file that fails to read back is not overwritten by the next save without a trace:
//! [`set_aside`] moves it to a `.bad-<seconds>` name first, so a person can still recover it.

use std::io;
use std::path::Path;

use base_fs::{NewFileMode, move_aside, write_atomic};
use base_time::{Clock, SystemClock};

use crate::error::ServiceError;

/// Replace the file at `path` (its folder created) with `bytes`, durably and atomically.
pub fn write(path: &Path, bytes: &[u8], new_file: NewFileMode) -> io::Result<()> {
    if let Some(folder) = path.parent().filter(|folder| !folder.as_os_str().is_empty()) {
        std::fs::create_dir_all(folder)?;
    }
    write_atomic(path, bytes, new_file).map_err(|error| error.source)
}

/// [`write`], naming the file in the error.
pub fn write_file(path: &Path, bytes: &[u8], new_file: NewFileMode) -> Result<(), ServiceError> {
    write(path, bytes, new_file).map_err(|source| ServiceError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Move the file at `path`, which could not be read back, to `<name>.bad-<seconds>` beside it, so
/// the next save does not destroy it. A file that is not there is nothing to set aside.
pub fn set_aside(path: &Path) {
    if !path.exists() {
        return;
    }
    if move_aside(path, SystemClock.now()).is_ok() {
        eprintln!(
            "bardo-shell: {} could not be read; kept aside as .bad-<seconds>",
            path.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::{PermissionsExt, symlink};

    use base_fs::NewFileMode;
    use shell_testkit::TempDir;

    use super::{set_aside, write};

    fn mode(path: &std::path::Path) -> u32 {
        std::fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o777)
            .unwrap_or_default()
    }

    fn names(dir: &std::path::Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    #[test]
    fn a_new_file_is_private_whole_and_leaves_nothing_beside_it() {
        let dir = TempDir::new("durable-new");
        let path = dir.path().join("state/bardo/shell/a.json");
        write(&path, b"one", NewFileMode::OwnerOnly).unwrap_or_else(|e| panic!("{e}"));
        write(&path, b"two", NewFileMode::OwnerOnly).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(std::fs::read(&path).unwrap_or_default(), b"two");
        assert_eq!(mode(&path), 0o600);
        assert_eq!(names(&dir.path().join("state/bardo/shell")), vec!["a.json"]);
    }

    #[test]
    fn a_file_keeps_the_permissions_it_had() {
        let dir = TempDir::new("durable-as-found");
        let path = dir.path().join("shared.toml");
        std::fs::write(&path, b"x").unwrap_or_else(|e| panic!("{e}"));
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
            .unwrap_or_else(|e| panic!("{e}"));
        write(&path, b"y", NewFileMode::OwnerOnly).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(mode(&path), 0o644);
    }

    #[test]
    fn a_link_in_the_files_place_is_written_through() {
        let dir = TempDir::new("durable-link");
        let real = dir.path().join("dotfiles/settings.toml");
        std::fs::create_dir_all(dir.path().join("dotfiles")).unwrap_or_else(|e| panic!("{e}"));
        std::fs::write(&real, b"old").unwrap_or_else(|e| panic!("{e}"));
        let link = dir.path().join("settings.toml");
        symlink(&real, &link).unwrap_or_else(|e| panic!("{e}"));
        write(&link, b"new", NewFileMode::OwnerOnly).unwrap_or_else(|e| panic!("{e}"));
        assert!(
            std::fs::symlink_metadata(&link)
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false),
            "the link is still a link"
        );
        assert_eq!(std::fs::read(&real).unwrap_or_default(), b"new");
    }

    #[test]
    fn a_file_that_could_not_be_read_is_kept_aside() {
        let dir = TempDir::new("durable-aside");
        let path = dir.path().join("a.json");
        std::fs::write(&path, b"{ broken").unwrap_or_else(|e| panic!("{e}"));
        set_aside(&path);
        assert!(!path.exists());
        let kept = names(dir.path());
        assert_eq!(kept.len(), 1, "{kept:?}");
        assert!(kept[0].starts_with("a.json.bad-"), "{kept:?}");
        assert_eq!(
            std::fs::read(dir.path().join(&kept[0])).unwrap_or_default(),
            b"{ broken"
        );
        set_aside(&path);
    }
}
