//! Temporary: delete with the old-folders crate in the release after everyone has started once.
//!
//! The moves the daemons of the actions domain make at start, from where the earlier release kept
//! its files to where this one does. Each daemon names its own old and new paths; this module only
//! finds the XDG bases the same way the daemon does and says what moved.

use base_xdg::Xdg;
use std::path::{Path, PathBuf};

/// `$XDG_CONFIG_HOME`, else `$HOME/.config`.
pub fn config_base(env: &impl Fn(&str) -> Option<String>) -> PathBuf {
    crate::callers::config_home(env)
}

/// `$XDG_DATA_HOME`, else `$HOME/.local/share`.
pub fn data_base(env: &impl Fn(&str) -> Option<String>) -> PathBuf {
    Xdg::new(env).data_home().unwrap_or_default()
}

/// Moves every entry of the folder `old` into `new`, once, and says so on stderr as `who`.
pub fn move_dir(who: &str, old: &Path, new: &Path) {
    match old_folders::move_once(old, new) {
        Ok(moved) if moved.moved > 0 => eprintln!(
            "{who}: moved {} entries of {} to {}",
            moved.moved,
            old.display(),
            new.display()
        ),
        Ok(_) => {}
        Err(why) => eprintln!("{who}: could not move {}: {why}", old.display()),
    }
}

/// Moves the file `old` to `new`, once, unless `new` exists, and says so on stderr as `who`.
pub fn move_file(who: &str, old: &Path, new: &Path) {
    match old_folders::move_file_once(old, new) {
        Ok(true) => eprintln!("{who}: moved {} to {}", old.display(), new.display()),
        Ok(false) => {}
        Err(why) => eprintln!("{who}: could not move {}: {why}", old.display()),
    }
}

/// Moves the person's callers file of the earlier release (`<config>/quire/<old_file>`) to
/// `<config>/bardo/actions/<new_file>`.
pub fn move_callers(
    who: &str,
    old_file: &str,
    new_file: &str,
    env: &impl Fn(&str) -> Option<String>,
) {
    let config = config_base(env);
    move_file(
        who,
        &config.join("quire").join(old_file),
        &config.join("bardo").join("actions").join(new_file),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_moves_once_into_a_scratch_folder_and_is_not_moved_over() {
        let dir = tempfile::tempdir().expect("scratch");
        let old = dir.path().join("quire/a.toml");
        let new = dir.path().join("bardo/actions/b.toml");
        std::fs::create_dir_all(old.parent().expect("parent")).expect("old folder");
        std::fs::write(&old, "old").expect("old file");
        move_file("test", &old, &new);
        assert_eq!(std::fs::read_to_string(&new).expect("moved"), "old");
        assert!(!old.exists());
        std::fs::write(&old, "again").expect("old file again");
        move_file("test", &old, &new);
        assert_eq!(std::fs::read_to_string(&new).expect("kept"), "old");
        assert!(old.exists());
    }

    #[test]
    fn a_folder_moves_into_the_new_one() {
        let dir = tempfile::tempdir().expect("scratch");
        let old = dir.path().join("docket");
        let new = dir.path().join("bardo/actions");
        std::fs::create_dir_all(&old).expect("old folder");
        std::fs::write(old.join("agents.toml"), "x").expect("old file");
        move_dir("test", &old, &new);
        assert!(new.join("agents.toml").is_file());
        assert!(!old.exists());
    }
}
