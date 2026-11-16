//! Temporary: delete with the old-folders crate in the release after everyone has started once.
//!
//! memoryd's folders had other names before the desktop was renamed; at start it moves what the old
//! folders hold into the new ones (see [`memory_core::Dirs`]), before it reads anything and before
//! its sandbox is closed around the new folders.

use crate::xdg::{XdgError, roots};
use std::path::PathBuf;

/// One thing to move: a folder's entries, or a single file.
enum Move {
    Folder(PathBuf, PathBuf),
    File(PathBuf, PathBuf),
}

/// What moves, for the roots `lookup` names (the same ones [`crate::dirs_from`] reads).
fn moves(lookup: &impl Fn(&str) -> Option<String>) -> Result<Vec<Move>, XdgError> {
    let [data, cache, config, runtime] = roots(lookup)?;
    let rules = config.join("bardo/memoryd");
    Ok(vec![
        Move::Folder(config.join("almanac"), config.join("bardo/memory")),
        Move::File(config.join("quire/memory.toml"), rules.join("memory.toml")),
        Move::File(
            config.join("quire/memoryd-callers.toml"),
            rules.join("callers.toml"),
        ),
        Move::File(
            config.join("quire/memory-callers.toml"),
            rules.join("callers.toml"),
        ),
        Move::Folder(data.join("quire/memory"), data.join("bardo/memory")),
        Move::Folder(cache.join("quire/memory"), cache.join("bardo/memory")),
        Move::Folder(runtime.join("quire/memory"), runtime.join("bardo/memory")),
    ])
}

/// Temporary: delete with the old-folders crate in the release after everyone has started once.
///
/// Moves memoryd's old folders and files into the new ones, never overwriting, and returns one
/// line for each thing that moved or could not be moved, for the log.
pub fn move_pre_bardo_folders(lookup: &impl Fn(&str) -> Option<String>) -> Vec<String> {
    let Ok(moves) = moves(lookup) else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    for item in moves {
        let (old, new, outcome) = match item {
            Move::Folder(old, new) => {
                let outcome = old_folders::move_once(&old, &new).map(|done| done.moved > 0);
                (old, new, outcome)
            }
            Move::File(old, new) => {
                let outcome = old_folders::move_file_once(&old, &new);
                (old, new, outcome)
            }
        };
        match outcome {
            Ok(true) => lines.push(format!(
                "memoryd: moved {} to {}",
                old.display(),
                new.display()
            )),
            Ok(false) => {}
            Err(why) => lines.push(format!(
                "memoryd: could not move {} to {}: {why}",
                old.display(),
                new.display()
            )),
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn the_old_folders_and_files_move_once_and_nothing_is_overwritten() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().to_owned();
        let lookup = |name: &str| match name {
            "HOME" => Some(home.display().to_string()),
            "XDG_RUNTIME_DIR" => Some(home.join("run").display().to_string()),
            _ => None,
        };
        let old_data = home.join(".local/share/quire/memory/desktop");
        fs::create_dir_all(&old_data).unwrap();
        fs::write(old_data.join("events.db"), "old").unwrap();
        fs::create_dir_all(home.join(".config/almanac")).unwrap();
        fs::write(home.join(".config/almanac/settings.toml"), "old").unwrap();
        fs::create_dir_all(home.join(".config/quire")).unwrap();
        fs::write(home.join(".config/quire/memory.toml"), "old").unwrap();
        fs::write(home.join(".config/quire/memoryd-callers.toml"), "old").unwrap();

        let lines = move_pre_bardo_folders(&lookup);

        assert_eq!(lines.len(), 4, "{lines:?}");
        let read = |path: &str| fs::read_to_string(home.join(path)).unwrap();
        assert_eq!(read(".local/share/bardo/memory/desktop/events.db"), "old");
        assert_eq!(read(".config/bardo/memory/settings.toml"), "old");
        assert_eq!(read(".config/bardo/memoryd/memory.toml"), "old");
        assert_eq!(read(".config/bardo/memoryd/callers.toml"), "old");
        assert!(!home.join(".local/share/quire/memory").exists());
        assert!(move_pre_bardo_folders(&lookup).is_empty());
    }

    #[test]
    fn the_new_places_are_the_ones_dirs_names() {
        let lookup = |name: &str| match name {
            "HOME" => Some("/h".to_owned()),
            "XDG_RUNTIME_DIR" => Some("/r".to_owned()),
            _ => None,
        };
        let dirs = crate::dirs_from(lookup).expect("dirs");
        let news: Vec<PathBuf> = moves(&lookup)
            .expect("moves")
            .into_iter()
            .map(|item| match item {
                Move::Folder(_, new) | Move::File(_, new) => new,
            })
            .collect();
        for expected in [
            dirs.memory(),
            dirs.cache_memory(),
            dirs.runtime_memory(),
            dirs.settings(),
            dirs.memory_toml(),
            dirs.config().join("callers.toml"),
        ] {
            assert!(news.contains(&expected), "{}", expected.display());
        }
    }
}
