//! How layer-shell-host sets the lock surfaces up: their name prefix, `shell-lock`, so each is
//! `shell-lock/<output>` (what `bardo-shell debug capture` and `debug stats` address).

use layer_shell_host::LockConfig;
use shell_settings::ShellFile;

/// The lock surfaces' configuration; nothing in the settings changes it yet.
pub fn config(_settings: &ShellFile) -> LockConfig {
    LockConfig {
        namespace: "shell-lock".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use shell_settings::ShellFile;

    use super::config;

    #[test]
    fn the_lock_surfaces_are_named_sill_lock() {
        assert_eq!(config(&ShellFile::default()).namespace, "shell-lock");
    }
}
