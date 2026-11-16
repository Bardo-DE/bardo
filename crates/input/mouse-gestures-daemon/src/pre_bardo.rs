//! Temporary: delete with the old-folders crate in the release after everyone has started once.
//!
//! The config folder of a run by a person sat at `$XDG_CONFIG_HOME/palmrest` before the rename. At
//! start it moves to `bardo/mouse-gestures` once, before the config is read. The unit's own folder
//! (`/etc`, from `ConfigurationDirectory=`) belongs to systemd and is moved by the installer.

use std::path::{Path, PathBuf};

/// Temporary: delete with the old-folders crate in the release after everyone has started once.
///
/// Moves `<config home>/palmrest` to `<config home>/bardo/mouse-gestures` when `lookup` has no
/// `CONFIGURATION_DIRECTORY` (a run by a person, not by the unit). What moved is logged.
pub fn move_pre_bardo_folders(lookup: impl Fn(&str) -> Option<String>) {
    if lookup("CONFIGURATION_DIRECTORY").is_some_and(|dir| !dir.is_empty()) {
        return;
    }
    let Some(base) = config_home(&lookup) else {
        return;
    };
    let (old, new) = (base.join("palmrest"), base.join("bardo/mouse-gestures"));
    match old_folders::move_once(&old, &new) {
        Ok(moved) if moved.moved > 0 => eprintln!(
            "bardo-mouse-gestures: moved {} entries from {} to {}",
            moved.moved,
            old.display(),
            new.display()
        ),
        Ok(_) => {}
        Err(error) => eprintln!(
            "bardo-mouse-gestures: could not move {} to {}: {error}",
            old.display(),
            new.display()
        ),
    }
}

fn config_home(lookup: &impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    lookup("XDG_CONFIG_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            lookup("HOME")
                .filter(|s| !s.is_empty())
                .map(|home| Path::new(&home).join(".config"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_persons_old_config_folder_moves_and_the_units_is_left_alone() {
        let root = std::env::temp_dir().join(format!("bardo-mg-pre-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("palmrest")).expect("create");
        std::fs::write(root.join("palmrest/gestures.toml"), "x").expect("write");
        let base = root.to_string_lossy().into_owned();

        move_pre_bardo_folders(|k| match k {
            "CONFIGURATION_DIRECTORY" => Some("/etc/bardo/mouse-gestures".to_owned()),
            "XDG_CONFIG_HOME" => Some(base.clone()),
            _ => None,
        });
        assert!(
            root.join("palmrest/gestures.toml").exists(),
            "the unit's run moves nothing"
        );

        move_pre_bardo_folders(|k| (k == "XDG_CONFIG_HOME").then(|| base.clone()));
        assert!(root.join("bardo/mouse-gestures/gestures.toml").exists());
        assert!(!root.join("palmrest").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
