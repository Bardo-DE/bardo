//! How shell-host sets the lock surfaces up: their name prefix, `sill-lock`, so each is
//! `sill-lock/<output>` (what `sill debug capture` and `debug stats` address).

use layer_shell_host::LockConfig;
use shell_settings::ShellFile;

/// The lock surfaces' configuration; nothing in the settings changes it yet.
pub fn config(_settings: &ShellFile) -> LockConfig {
    LockConfig {
        namespace: "sill-lock".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use shell_settings::ShellFile;

    use super::config;

    #[test]
    fn the_lock_surfaces_are_named_sill_lock() {
        assert_eq!(config(&ShellFile::default()).namespace, "sill-lock");
    }
}
