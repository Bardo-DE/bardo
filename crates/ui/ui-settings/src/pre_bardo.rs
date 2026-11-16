//! Temporary: delete with the old-folders crate in the release after everyone has started once.
//!
//! The folders the desktop's programs share (the look, the extensions, the installed schemas,
//! intents, helpers and icons) sat under `quire` in each XDG base; they now sit under `bardo`.
//! A program of the desktop calls [`move_shared_folders`] once at start, before it reads anything,
//! and each entry moves only when the new folder lacks it, so the first program to start does the
//! move and the others find nothing to do.

use std::path::{Path, PathBuf};

use crate::xdg;

/// The XDG bases the moves are made under.
#[derive(Debug, Clone, Default)]
pub struct Bases {
    /// `$XDG_CONFIG_HOME`, else `$HOME/.config`.
    pub config: Option<PathBuf>,
    /// `$XDG_DATA_HOME`, else `$HOME/.local/share`.
    pub data: Option<PathBuf>,
    /// `$XDG_STATE_HOME`, else `$HOME/.local/state`.
    pub state: Option<PathBuf>,
    /// `$XDG_CACHE_HOME`, else `$HOME/.cache`.
    pub cache: Option<PathBuf>,
    /// `$XDG_RUNTIME_DIR`, when it is an absolute path.
    pub runtime: Option<PathBuf>,
}

impl Bases {
    /// The bases of this process's environment, by the XDG rules of [`crate::xdg`].
    #[must_use]
    pub fn from_env() -> Bases {
        let home = std::env::var_os("HOME");
        let base = |name: &str, under_home: &str| {
            xdg::base_dir(
                std::env::var_os(name).as_deref(),
                home.as_deref(),
                under_home,
            )
        };
        Bases {
            config: base("XDG_CONFIG_HOME", xdg::CONFIG_HOME_UNDER_HOME),
            data: base("XDG_DATA_HOME", xdg::DATA_HOME_UNDER_HOME),
            state: base("XDG_STATE_HOME", xdg::STATE_HOME_UNDER_HOME),
            cache: base("XDG_CACHE_HOME", ".cache"),
            runtime: std::env::var_os("XDG_RUNTIME_DIR")
                .map(PathBuf::from)
                .filter(|dir| dir.is_absolute()),
        }
    }
}

/// The files of the old `quire` config folder that are the desktop-wide settings.
const CONFIG_FILES: [&str; 4] = [
    "appearance.toml",
    "style.css",
    "extensions.toml",
    "spaces.json",
];
/// The folders of the old `quire` config folder that are the desktop-wide settings.
const CONFIG_FOLDERS: [&str; 1] = ["selectors"];
/// The folders of the old `quire` data folder every program of the desktop reads.
const DATA_FOLDERS: [&str; 8] = [
    "intents",
    "settings",
    "helpers",
    "extensions",
    "skills",
    "icons",
    "selectors",
    "spelling",
];

/// Temporary: delete with the old-folders crate in the release after everyone has started once.
///
/// Moves what `quire` held in the config, data and state bases (the look files, the extensions,
/// the schema, intents, helpers, skills and icons folders, the styles guard) and the emoji font's
/// `fonts/quire` folder to `bardo`. `who` names the program in the log lines.
pub fn move_shared_folders(who: &str, bases: &Bases) {
    if let Some(config) = &bases.config {
        let (old, new) = (config.join("quire"), config.join("bardo"));
        for name in CONFIG_FILES {
            move_file(who, &old.join(name), &new.join(name));
        }
        for name in CONFIG_FOLDERS {
            move_folder(who, &old.join(name), &new.join(name));
        }
    }
    if let Some(data) = &bases.data {
        let (old, new) = (data.join("quire"), data.join("bardo"));
        for name in DATA_FOLDERS {
            move_folder(who, &old.join(name), &new.join(name));
        }
        move_folder(who, &data.join("fonts/quire"), &data.join("fonts/bardo"));
    }
    if let Some(state) = &bases.state {
        move_folder(
            who,
            &state.join("quire/styles-guard"),
            &state.join("bardo/styles-guard"),
        );
    }
}

/// Moves every entry of `old` into `new` once, and says on stderr what moved or why not. A folder
/// that is not there is nothing to do.
pub fn move_folder(who: &str, old: &Path, new: &Path) {
    match old_folders::move_once(old, new) {
        Ok(done) if done.moved > 0 => eprintln!(
            "{who}: moved {} entries from {} to {}",
            done.moved,
            old.display(),
            new.display()
        ),
        Ok(_) => {}
        Err(error) => eprintln!(
            "{who}: could not move {} to {}: {error}",
            old.display(),
            new.display()
        ),
    }
}

/// Moves the file `old` to `new` once, unless `new` exists, and says on stderr what moved or why not.
pub fn move_file(who: &str, old: &Path, new: &Path) {
    match old_folders::move_file_once(old, new) {
        Ok(true) => eprintln!("{who}: moved {} to {}", old.display(), new.display()),
        Ok(false) => {}
        Err(error) => eprintln!(
            "{who}: could not move {} to {}: {error}",
            old.display(),
            new.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, relative: &str, text: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create");
        std::fs::write(path, text).expect("write");
    }

    fn read(root: &Path, relative: &str) -> String {
        std::fs::read_to_string(root.join(relative)).expect("read")
    }

    #[test]
    fn the_shared_folders_move_under_bardo_and_nothing_is_overwritten() {
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let root = scratch.path();
        let bases = Bases {
            config: Some(root.join("config")),
            data: Some(root.join("data")),
            state: Some(root.join("state")),
            cache: None,
            runtime: None,
        };
        write(root, "config/quire/appearance.toml", "old look");
        write(root, "config/quire/style.css", "old style");
        write(root, "config/bardo/style.css", "new style");
        write(root, "config/quire/intentd.toml", "not shared");
        write(root, "data/quire/settings/shell.settings.toml", "schema");
        write(root, "data/quire/memory/log", "not shared");
        write(root, "data/fonts/quire/Noto-COLRv1.ttf", "font");
        write(root, "state/quire/styles-guard/shell", "guard");
        write(root, "state/quire/accountd/audit.jsonl", "not shared");

        move_shared_folders("test", &bases);

        assert_eq!(read(root, "config/bardo/appearance.toml"), "old look");
        assert_eq!(read(root, "config/bardo/style.css"), "new style");
        assert_eq!(read(root, "config/quire/style.css"), "old style");
        assert_eq!(read(root, "config/quire/intentd.toml"), "not shared");
        assert_eq!(
            read(root, "data/bardo/settings/shell.settings.toml"),
            "schema"
        );
        assert_eq!(read(root, "data/quire/memory/log"), "not shared");
        assert_eq!(read(root, "data/fonts/bardo/Noto-COLRv1.ttf"), "font");
        assert_eq!(read(root, "state/bardo/styles-guard/shell"), "guard");
        assert_eq!(read(root, "state/quire/accountd/audit.jsonl"), "not shared");
    }
}
