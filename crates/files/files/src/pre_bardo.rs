//! Temporary: delete with the old-folders crate in the release after everyone has started once.
//!
//! The file manager's folders sat under `files` in each XDG base (its settings and style in the
//! config base, the folder views it remembers in the state base); they now sit under
//! `bardo/files`. At start they are moved once, before anything reads them.

use ui_settings::AppName;
use ui_settings::pre_bardo::{Bases, move_folder};

/// The file manager's name in each XDG base (`<base>/bardo/files`).
const APP: AppName = AppName::desktop("files");
/// How the move names the program in the log.
const WHO: &str = "bardo-files";

/// Temporary: delete with the old-folders crate in the release after everyone has started once.
///
/// Moves `files` in the config and state bases to `<base>/bardo/files`, and the folders every
/// program of the desktop shares from `quire` to `bardo`. Each entry moves only when the new folder
/// lacks it; what moved is logged.
pub fn move_pre_bardo_folders(bases: &Bases) {
    for base in [&bases.config, &bases.state].into_iter().flatten() {
        move_folder(WHO, &base.join("files"), &APP.under(base));
    }
    ui_settings::pre_bardo::move_shared_folders(WHO, bases);
}

/// [`move_pre_bardo_folders`] for the folders of this process's environment.
pub fn move_from_env() {
    move_pre_bardo_folders(&Bases::from_env());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_old_folders_move_under_bardo_files_and_nothing_is_overwritten() {
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let root = scratch.path();
        let bases = Bases {
            config: Some(root.join("config")),
            data: None,
            state: Some(root.join("state")),
            cache: None,
            runtime: None,
        };
        let write = |path: &str, text: &str| {
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().expect("a parent")).expect("create");
            std::fs::write(file, text).expect("write");
        };
        write("config/files/settings.toml", "old");
        write("state/files/folders.toml", "old views");
        write("state/bardo/files/folders.toml", "new views");

        move_pre_bardo_folders(&bases);

        let read = |path: &str| std::fs::read_to_string(root.join(path)).expect("read");
        assert_eq!(read("config/bardo/files/settings.toml"), "old");
        assert_eq!(read("state/bardo/files/folders.toml"), "new views");
        assert_eq!(read("state/files/folders.toml"), "old views");
        assert!(!root.join("config/files").exists());
    }
}
