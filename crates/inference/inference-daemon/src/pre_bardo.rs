//! Temporary: delete with the old-folders crate in the release after everyone has started once.
//!
//! inferd's folders had other names before the desktop was renamed; at start it moves what the old
//! folders hold into the new ones (see [`crate::config::Dirs`]), before it reads anything.
//! The accounts folders (`porter/providers`, `porter/clients.toml`) are accountd's to move.

use crate::config::ConfigError;
use base_xdg::{PathError, Xdg};
use std::path::PathBuf;

/// The folders of the variables `var` answers, before and after the rename: `(old, new)` pairs.
fn pairs(var: impl Fn(&str) -> Option<String>) -> Result<Vec<(PathBuf, PathBuf)>, ConfigError> {
    let xdg = Xdg::new(var);
    let dir = |name: &str, fallback: &str| {
        xdg.dir(name, fallback)
            .map_err(|_: PathError| ConfigError::NoDirs)
    };
    let config = dir("XDG_CONFIG_HOME", ".config")?;
    let data = dir("XDG_DATA_HOME", ".local/share")?;
    let state = dir("XDG_STATE_HOME", ".local/state")?;
    Ok(vec![
        (config.join("inferd"), config.join("bardo/inference")),
        (state.join("quire/inferd"), state.join("bardo/inference")),
        (data.join("porter/engines"), data.join("bardo/engines")),
        (
            data.join("stoker/catalog"),
            data.join("bardo/models/catalog"),
        ),
    ])
}

/// Temporary: delete with the old-folders crate in the release after everyone has started once.
///
/// Moves the old folders of inferd into the new ones, never overwriting an entry, and returns one
/// line for each folder that moved something or could not be moved, for the log.
pub fn move_pre_bardo_folders(var: impl Fn(&str) -> Option<String>) -> Vec<String> {
    let Ok(pairs) = pairs(var) else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    for (old, new) in pairs {
        match old_folders::move_once(&old, &new) {
            Ok(moved) if moved.moved > 0 => lines.push(format!(
                "inferd: moved {} entries from {} to {}",
                moved.moved,
                old.display(),
                new.display()
            )),
            Ok(_) => {}
            Err(why) => lines.push(format!(
                "inferd: could not move {} to {}: {why}",
                old.display(),
                new.display()
            )),
        }
    }
    lines
}

/// The new folders in `dirs` must be where the pairs above end; a test keeps the two in step.
#[cfg(test)]
fn new_folders(dirs: &crate::config::Dirs) -> [PathBuf; 3] {
    [
        dirs.config.parent().map(PathBuf::from).unwrap_or_default(),
        dirs.engines_own.clone(),
        dirs.catalog.user.clone(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Dirs;
    use std::fs;

    #[test]
    fn the_old_folders_move_into_the_new_ones_and_nothing_is_overwritten() {
        let scratch = base_testkit::Scratch::new("inferd-pre-bardo");
        let root = scratch.path().to_owned();
        let vars = {
            let root = root.clone();
            move |name: &str| match name {
                "HOME" => Some(root.display().to_string()),
                "XDG_RUNTIME_DIR" => Some(root.join("run").display().to_string()),
                _ => None,
            }
        };
        let old_config = root.join(".config/inferd");
        let old_state = root.join(".local/state/quire/inferd");
        let new_state = root.join(".local/state/bardo/inference");
        fs::create_dir_all(&old_config).unwrap();
        fs::create_dir_all(&old_state).unwrap();
        fs::create_dir_all(&new_state).unwrap();
        fs::write(old_config.join("inferd.toml"), "old").unwrap();
        fs::write(old_state.join("spend.json"), "old").unwrap();
        fs::write(old_state.join("audit.jsonl"), "old").unwrap();
        fs::write(new_state.join("audit.jsonl"), "new").unwrap();

        let lines = move_pre_bardo_folders(&vars);

        assert_eq!(lines.len(), 2, "{lines:?}");
        assert_eq!(
            fs::read_to_string(root.join(".config/bardo/inference/inferd.toml")).unwrap(),
            "old"
        );
        assert!(!old_config.exists());
        assert_eq!(
            fs::read_to_string(new_state.join("spend.json")).unwrap(),
            "old"
        );
        assert_eq!(
            fs::read_to_string(new_state.join("audit.jsonl")).unwrap(),
            "new"
        );
        assert!(
            old_state.join("audit.jsonl").exists(),
            "the clash stays where it was"
        );
        assert!(
            move_pre_bardo_folders(&vars).is_empty(),
            "a second start moves nothing"
        );
    }

    #[test]
    fn the_new_folders_are_the_ones_dirs_names() {
        let vars = |name: &str| match name {
            "HOME" => Some("/h".to_owned()),
            "XDG_RUNTIME_DIR" => Some("/r".to_owned()),
            _ => None,
        };
        let dirs = Dirs::from_vars(vars).expect("dirs");
        let [config, engines, catalog] = new_folders(&dirs);
        let moves = pairs(vars).expect("pairs");
        let news: Vec<&PathBuf> = moves.iter().map(|(_, new)| new).collect();
        assert!(news.contains(&&config));
        assert!(news.contains(&&engines));
        assert!(news.contains(&&catalog));
        assert!(news.contains(&&dirs.audit.parent().unwrap().to_path_buf()));
    }
}
