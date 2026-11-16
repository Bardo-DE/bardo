//! Temporary: delete with the old-folders crate in the release after everyone has started once.
//!
//! The Settings app's own folders sat under `detent` in each XDG base (the first-run mark in the
//! state base, the pictures the person added in the config and data bases); they now sit under
//! `bardo/settings`. At start they are moved once, before anything reads them.

use ui_settings::AppName;
use ui_settings::pre_bardo::{Bases, move_folder};

/// The Settings app's name in each XDG base (`<base>/bardo/settings`).
const APP: AppName = AppName::desktop("settings");
/// The key remapper's name in each XDG base (`<base>/bardo/key-remap`).
const KEY_REMAP: AppName = AppName::desktop("key-remap");
/// How the move names the program in the log.
const WHO: &str = "bardo-settings";

/// Temporary: delete with the old-folders crate in the release after everyone has started once.
///
/// Moves `detent` in the config, data and state bases to `<base>/bardo/settings`, `config/keycap`
/// (the keymap the Settings app edits) to `<config>/bardo/key-remap`, and the folders every
/// program of the desktop shares from `quire` to `bardo`. Each entry moves only when the new folder
/// lacks it; what moved is logged.
pub fn move_pre_bardo_folders(bases: &Bases) {
    for base in [&bases.config, &bases.data, &bases.state]
        .into_iter()
        .flatten()
    {
        move_folder(WHO, &base.join("detent"), &APP.under(base));
    }
    if let Some(config) = &bases.config {
        move_folder(WHO, &config.join("keycap"), &KEY_REMAP.under(config));
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
    fn the_old_folders_move_under_bardo_settings_and_nothing_is_overwritten() {
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let root = scratch.path();
        let bases = Bases {
            config: Some(root.join("config")),
            data: Some(root.join("data")),
            state: Some(root.join("state")),
            cache: None,
            runtime: None,
        };
        let write = |path: &str, text: &str| {
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().expect("a parent")).expect("create");
            std::fs::write(file, text).expect("write");
        };
        write("state/detent/first-run", "done\n");
        write("config/detent/pictures/a.png", "picture");
        write("config/keycap/keymap", "keys");
        write("config/bardo/key-remap/keymap", "new keys");

        move_pre_bardo_folders(&bases);

        let read = |path: &str| std::fs::read_to_string(root.join(path)).expect("read");
        assert_eq!(read("state/bardo/settings/first-run"), "done\n");
        assert_eq!(read("config/bardo/settings/pictures/a.png"), "picture");
        assert_eq!(read("config/bardo/key-remap/keymap"), "new keys");
        assert_eq!(read("config/keycap/keymap"), "keys");
        assert!(!root.join("state/detent").exists());
    }
}
