//! Temporary: delete with the old-folders crate in the release after everyone has started once.
//!
//! The compositor's folders, and the keymap folder it reads, sat under their pre-bardo names
//! (`casement`, `keycap`). At start they are moved to their new places once, before anything reads
//! them.

use std::path::{Path, PathBuf};
use ui_settings::AppName;

/// The compositor's own name in each XDG base (`<base>/bardo/compositor`).
const APP: AppName = AppName::desktop("compositor");
/// The key remapper's name in each XDG base (`<base>/bardo/key-remap`).
const KEY_REMAP: AppName = AppName::desktop("key-remap");

/// The XDG bases the moves are made under.
#[derive(Debug, Clone, Default)]
pub struct Bases {
    /// `$XDG_CONFIG_HOME`, else `$HOME/.config`.
    pub config: Option<PathBuf>,
    /// `$XDG_STATE_HOME`, else `$HOME/.local/state`.
    pub state: Option<PathBuf>,
    /// `$XDG_RUNTIME_DIR`.
    pub runtime: Option<PathBuf>,
}

impl Bases {
    /// The bases of this process's environment.
    #[must_use]
    pub fn from_env() -> Bases {
        let var = |name: &str| {
            std::env::var_os(name)
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
        };
        let home = var("HOME");
        let under_home = |dir: &str| home.as_ref().map(|home| home.join(dir));
        Bases {
            config: var("XDG_CONFIG_HOME").or_else(|| under_home(".config")),
            state: var("XDG_STATE_HOME").or_else(|| under_home(".local/state")),
            runtime: var("XDG_RUNTIME_DIR"),
        }
    }
}

/// Temporary: delete with the old-folders crate in the release after everyone has started once.
///
/// Moves `config/casement` (settings), `state/casement` (Spaces, privacy book) and
/// `runtime/casement` (lock marker) to `<base>/bardo/compositor`, and `config/keycap` (the keymap) to
/// `<config>/bardo/key-remap`, each entry only when the new folder lacks it. What moved is logged.
pub fn move_pre_bardo_folders(bases: &Bases) {
    let moves = [
        (&bases.config, "casement", APP),
        (&bases.state, "casement", APP),
        (&bases.runtime, "casement", APP),
        (&bases.config, "keycap", KEY_REMAP),
    ];
    for (base, old, app) in moves {
        if let Some(base) = base {
            move_one(&base.join(old), &app.under(base));
        }
    }
}

fn move_one(old: &Path, new: &Path) {
    match old_folders::move_once(old, new) {
        Ok(moved) => {
            if moved.moved > 0 {
                eprintln!(
                    "bardo-compositor: moved {} entries from {} to {}",
                    moved.moved,
                    old.display(),
                    new.display()
                );
            }
        }
        Err(error) => eprintln!(
            "bardo-compositor: could not move {} to {}: {error}",
            old.display(),
            new.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_old_folders_move_under_the_bardo_level_and_nothing_is_overwritten() {
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let root = scratch.path();
        let bases = Bases {
            config: Some(root.join("config")),
            state: Some(root.join("state")),
            runtime: Some(root.join("run")),
        };
        std::fs::create_dir_all(root.join("config/casement")).expect("create");
        std::fs::write(root.join("config/casement/settings.toml"), "old").expect("write");
        std::fs::create_dir_all(root.join("config/keycap")).expect("create");
        std::fs::write(root.join("config/keycap/keymap"), "keys").expect("write");
        std::fs::create_dir_all(root.join("state/casement")).expect("create");
        std::fs::write(root.join("state/casement/spaces.toml"), "spaces").expect("write");
        std::fs::create_dir_all(root.join("state/bardo/compositor")).expect("create");
        std::fs::write(root.join("state/bardo/compositor/spaces.toml"), "new").expect("write");

        move_pre_bardo_folders(&bases);

        let read = |path: &str| std::fs::read_to_string(root.join(path)).expect("read");
        assert_eq!(read("config/bardo/compositor/settings.toml"), "old");
        assert_eq!(read("config/bardo/key-remap/keymap"), "keys");
        assert_eq!(read("state/bardo/compositor/spaces.toml"), "new");
        assert!(!root.join("config/casement").exists());
        assert!(!root.join("config/keycap").exists());
        assert!(
            root.join("state/casement/spaces.toml").exists(),
            "an entry the new folder has stays behind"
        );
    }
}
