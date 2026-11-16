//! Temporary: delete with the old-folders crate in the release after everyone has started once.
//!
//! The shell's folders sat under `sill` in each XDG base (and its keymap layer under `keycap`);
//! they now sit under `bardo/shell` (and `bardo/key-remap`). At start they are moved once, before
//! anything reads them.

use ui_settings::AppName;
use ui_settings::pre_bardo::{Bases, move_folder};

/// The shell's own name in each XDG base (`<base>/bardo/shell`).
const APP: AppName = AppName::desktop("shell");
/// The key remapper's name in each XDG base (`<base>/bardo/key-remap`).
const KEY_REMAP: AppName = AppName::desktop("key-remap");
/// How the move names the program in the log.
const WHO: &str = "bardo-shell";

/// Temporary: delete with the old-folders crate in the release after everyone has started once.
///
/// Moves `config/sill` (settings, layout, wallpaper source), `data/sill` (calendars), `state/sill`
/// (frecency, notification history), `cache/sill` (icons, art, shots, notifications) and
/// `runtime/sill` (the lock marker) to `<base>/bardo/shell`, `config/keycap` (the keymap layer) to
/// `<config>/bardo/key-remap`, and the folders every program of the desktop shares from `quire`
/// to `bardo`. Each entry moves only when the new folder lacks it; what moved is logged.
pub fn move_pre_bardo_folders(bases: &Bases) {
    let own = [
        &bases.config,
        &bases.data,
        &bases.state,
        &bases.cache,
        &bases.runtime,
    ];
    for base in own.into_iter().flatten() {
        move_folder(WHO, &base.join("sill"), &APP.under(base));
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
    fn the_old_folders_move_under_bardo_shell_and_nothing_is_overwritten() {
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let root = scratch.path();
        let bases = Bases {
            config: Some(root.join("config")),
            data: Some(root.join("data")),
            state: Some(root.join("state")),
            cache: Some(root.join("cache")),
            runtime: Some(root.join("run")),
        };
        let write = |path: &str, text: &str| {
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().expect("a parent")).expect("create");
            std::fs::write(file, text).expect("write");
        };
        write("config/sill/settings.toml", "old");
        write("config/keycap/keymap", "keys");
        write("data/sill/calendars/a.ics", "calendar");
        write("state/sill/frecency.json", "old frecency");
        write("state/bardo/shell/frecency.json", "new frecency");
        write("cache/sill/icons/a.png", "icon");
        write("run/sill/locked", "1");

        move_pre_bardo_folders(&bases);

        let read = |path: &str| std::fs::read_to_string(root.join(path)).expect("read");
        assert_eq!(read("config/bardo/shell/settings.toml"), "old");
        assert_eq!(read("config/bardo/key-remap/keymap"), "keys");
        assert_eq!(read("data/bardo/shell/calendars/a.ics"), "calendar");
        assert_eq!(read("state/bardo/shell/frecency.json"), "new frecency");
        assert_eq!(read("cache/bardo/shell/icons/a.png"), "icon");
        assert_eq!(read("run/bardo/shell/locked"), "1");
        assert!(!root.join("config/sill").exists());
        assert_eq!(
            read("state/sill/frecency.json"),
            "old frecency",
            "an entry the new folder has stays behind"
        );
    }
}
