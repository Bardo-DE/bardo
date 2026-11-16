//! Moves a service's folders from their pre-bardo paths (`~/.config/porter`, `~/.local/state/sill`, ...)
//! to their bardo paths, once, when the service starts.
//!
//! Temporary by design: the desktop was renamed in one release (owner decision 2026-11-16: "move once,
//! then delete"), so this crate and every call to it are deleted in a later release, once each machine
//! has started the renamed services once.
//!
//! The move is per entry and never overwrites: an entry that already exists at the new path wins and the
//! old one is left where it is, so running it again, or after a partial move, is harmless.

use std::io;
use std::path::Path;

/// What [`move_once`] did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Moved {
    /// Entries moved from the old folder into the new one.
    pub moved: usize,
    /// Entries left in the old folder because the new folder already had one of that name.
    pub kept: usize,
}

/// Moves every entry of `old` into `new` (created if missing), skipping names `new` already has, and
/// removes `old` once it is empty. Nothing happens when `old` does not exist.
///
/// Entries are renamed, not copied, so `old` and `new` should be on one file system (they are: both sit
/// under the same XDG base). A rename that crosses file systems fails with the error from `rename`.
pub fn move_once(old: &Path, new: &Path) -> io::Result<Moved> {
    let mut done = Moved::default();
    let entries = match std::fs::read_dir(old) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(done),
        Err(error) => return Err(error),
    };
    std::fs::create_dir_all(new)?;
    for entry in entries {
        let entry = entry?;
        let target = new.join(entry.file_name());
        if target.symlink_metadata().is_ok() {
            done.kept += 1;
            continue;
        }
        std::fs::rename(entry.path(), &target)?;
        done.moved += 1;
    }
    if done.kept == 0 {
        // Empty now; a failure here (something appeared meanwhile) leaves a harmless empty folder.
        let _ = std::fs::remove_dir(old);
    }
    Ok(done)
}

/// Moves one file from `old` to `new` (its folder created if missing) unless `new` already exists.
/// Returns whether it moved.
pub fn move_file_once(old: &Path, new: &Path) -> io::Result<bool> {
    if new.symlink_metadata().is_ok() || old.symlink_metadata().is_err() {
        return Ok(false);
    }
    if let Some(parent) = new.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(old, new)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_entries_keeps_existing_ones_and_removes_the_empty_old_folder() {
        let dir = tempfile::tempdir().expect("scratch");
        let old = dir.path().join("porter");
        let new = dir.path().join("bardo/accounts");
        std::fs::create_dir_all(old.join("providers")).expect("old");
        std::fs::write(old.join("callers.toml"), "old").expect("old file");
        std::fs::write(old.join("clients.toml"), "old").expect("old file");
        std::fs::create_dir_all(&new).expect("new");
        std::fs::write(new.join("clients.toml"), "new").expect("new file");

        let done = move_once(&old, &new).expect("move");
        assert_eq!(done, Moved { moved: 2, kept: 1 });
        assert_eq!(std::fs::read_to_string(new.join("clients.toml")).expect("kept"), "new");
        assert_eq!(std::fs::read_to_string(new.join("callers.toml")).expect("moved"), "old");
        assert!(new.join("providers").is_dir());
        assert!(old.exists(), "the old folder still holds the entry the new one already had");

        std::fs::remove_file(old.join("clients.toml")).expect("clear");
        assert_eq!(move_once(&old, &new).expect("again"), Moved::default());
        assert!(!old.exists(), "an empty old folder is removed");
    }

    #[test]
    fn a_missing_old_folder_is_nothing_to_do() {
        let dir = tempfile::tempdir().expect("scratch");
        let new = dir.path().join("new");
        assert_eq!(move_once(&dir.path().join("none"), &new).expect("move"), Moved::default());
        assert!(!new.exists());
    }

    #[test]
    fn a_file_moves_only_when_the_new_one_is_absent() {
        let dir = tempfile::tempdir().expect("scratch");
        let old = dir.path().join("inferd.toml");
        let new = dir.path().join("bardo/inference/inferd.toml");
        std::fs::write(&old, "old").expect("old");
        assert!(move_file_once(&old, &new).expect("move"));
        std::fs::write(&old, "again").expect("old again");
        assert!(!move_file_once(&old, &new).expect("second"));
        assert_eq!(std::fs::read_to_string(&new).expect("new"), "old");
    }
}
