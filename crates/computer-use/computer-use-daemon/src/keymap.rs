//! The keymap the computer-use daemon tells its model about: the platform's conventions and, where the `key_remap`
//! feature is on, this desktop's own bindings and the person's overrides, read once at start.
//!
//! The computer-use daemon uses the standard shortcuts only (owner decision, 2026-10-10). Its sandbox
//! (`dist/bardo-computer-use.service`) keeps the person's home and the key remap daemon socket closed, so their own
//! remappings are not read and the shortcut hints come from the chord keymap's conventional keymap (with
//! the desktop's own bindings). A source that cannot be opened is therefore not a problem: it is
//! skipped without a word, and only what was read and found wrong (a bad line in a keymap file
//! that did open) is reported.

use keymap::{Keymap, Platform};

/// Whether `problem` is what key-remap-client says of a keymap file it could not open.
///
/// That is `couldn't be used` and the operating system's own reason (`os error`). Under the
/// sandbox every closed source says it.
#[cfg(feature = "key-remap")]
fn is_closed_source(problem: &str) -> bool {
    problem.contains("couldn't be used") && problem.contains("(os error")
}

/// The keymap of `platform`, with every problem found reading it in plain words.
///
/// Nothing is reported without the `key_remap` feature. Environment variables are read through
/// `lookup`; the daemon's `main` passes the process's.
///
/// With the feature, the person's keymap comes from the key remap daemon's daemon, or its files when none
/// answers; a daemon or file that is closed to the computer-use daemon (its sandbox) leaves the conventional keymap
/// and no problem. Without the feature, only the platform's conventions apply.
#[cfg(feature = "key-remap")]
pub fn desktop_keymap(
    platform: Platform,
    lookup: impl Fn(&str) -> Option<String>,
) -> (Keymap, Vec<String>) {
    let (keymap, problems) =
        key_remap_client::KeyRemapSource::from_lookup(lookup).load_reporting(platform);
    let read = problems
        .into_iter()
        .filter(|problem| !is_closed_source(problem))
        .collect();
    (keymap, read)
}

/// The keymap of `platform`: its conventions only (the `key_remap` feature is off).
#[cfg(not(feature = "key-remap"))]
pub fn desktop_keymap(
    platform: Platform,
    lookup: impl Fn(&str) -> Option<String>,
) -> (Keymap, Vec<String>) {
    let _ = lookup;
    (Keymap::conventional(platform), Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use keymap::{Action, Desktop, StandardAction};

    const OURS: Platform = Platform::Linux {
        desktop: Desktop::Ours,
    };

    fn copy_of(keymap: &Keymap) -> Vec<String> {
        keymap
            .chords_of(&Action::Standard(StandardAction::Copy))
            .iter()
            .map(|chord| chord.display(OURS))
            .collect()
    }

    #[test]
    fn nothing_found_is_the_conventions() {
        let nowhere = |name: &str| match name {
            "BARDO_KEY_REMAP_SOCKET" => Some("/nonexistent/key-remap.sock".to_owned()),
            "BARDO_KEY_REMAP_CONFIG_DIR" => Some("/nonexistent/key_remap".to_owned()),
            _ => None,
        };
        let (keymap, _) = desktop_keymap(OURS, nowhere);
        assert!(copy_of(&keymap).contains(&"\u{2318}C".to_owned()));
    }

    #[cfg(feature = "key-remap")]
    #[test]
    fn the_persons_own_keymap_file_is_laid_over_the_conventions() {
        let home = tempfile::tempdir().expect("a scratch directory");
        let dir = home.path().join("bardo/key-remap");
        std::fs::create_dir_all(&dir).expect("the key-remap directory");
        std::fs::write(dir.join("keymap"), "copy = Primary+Alt+C\n").expect("the keymap file");
        let root = home.path().to_string_lossy().into_owned();
        let lookup = |name: &str| match name {
            "XDG_CONFIG_HOME" => Some(root.clone()),
            "BARDO_KEY_REMAP_SOCKET" => Some(format!("{root}/no-socket")),
            "BARDO_KEY_REMAP_CONFIG_DIR" => Some(format!("{root}/no-machine-dir")),
            _ => None,
        };
        let (keymap, _) = desktop_keymap(OURS, lookup);
        assert_eq!(copy_of(&keymap), ["\u{2325}\u{2318}C".to_owned()]);
    }

    #[cfg(feature = "key-remap")]
    #[test]
    fn sources_closed_to_computer_use_daemon_leave_the_conventions_and_say_nothing() {
        // A path below a regular file cannot be opened (not "missing"): what the sandbox does to
        // the person's home.
        let home = tempfile::tempdir().expect("a scratch directory");
        let blocker = home.path().join("closed");
        std::fs::write(&blocker, "x").expect("a plain file");
        let root = blocker.to_string_lossy().into_owned();
        let lookup = |name: &str| match name {
            "XDG_CONFIG_HOME" => Some(root.clone()),
            "BARDO_KEY_REMAP_SOCKET" => Some(format!("{root}/no-socket")),
            "BARDO_KEY_REMAP_CONFIG_DIR" => Some(format!("{root}/no-machine-dir")),
            _ => None,
        };
        let (keymap, problems) = desktop_keymap(OURS, lookup);
        assert_eq!(problems, Vec::<String>::new());
        assert!(copy_of(&keymap).contains(&"\u{2318}C".to_owned()));
    }

    #[cfg(feature = "key-remap")]
    #[test]
    fn a_closed_source_reads_as_closed_and_a_bad_line_does_not() {
        assert!(is_closed_source(
            "Your keymap: The keymap file couldn't be used (Permission denied (os error 13))."
        ));
        assert!(!is_closed_source("Your keymap: line 2: not a binding"));
        assert!(!is_closed_source(
            "Your keymap: The keymap file couldn't be used (it is larger than 64 KiB)."
        ));
    }
}
