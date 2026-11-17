//! `bardo-shell --write-selectors DIR`: the shell's public style parts as data, so Settings can
//! check a person's stylesheet against them without linking bardo-shell.
//!
//! The table lists only the parts a stylesheet may rely on; every other `shell-` class is internal.

use std::io;
use std::path::{Path, PathBuf};

use shell_settings::APP;
use ui_components::selectors::{AppSelectors, ComponentSelectors};
use ui_settings::PublishedParts;

/// The parts of the shell a stylesheet may name.
pub const SHELL_SELECTORS: AppSelectors = AppSelectors {
    app: "shell",
    components: &[
        ComponentSelectors {
            component: "Bar",
            root: "shell-bar",
            parts: &["group"],
        },
        ComponentSelectors {
            component: "Dock",
            root: "shell-dock",
            parts: &["hit"],
        },
        ComponentSelectors {
            component: "Switcher",
            root: "shell-switcher",
            parts: &["panel"],
        },
        ComponentSelectors {
            component: "Control Center",
            root: "shell-cc",
            parts: &["media", "detail"],
        },
    ],
    renames: &[],
};

/// The table as data, the file Settings reads.
#[must_use]
pub fn parts() -> PublishedParts {
    PublishedParts::new(APP.name(), SHELL_SELECTORS.classes())
}

/// Write `shell.toml` into `dir` (made if it is missing) and return its path.
///
/// # Errors
///
/// Returns the I/O error of creating `dir` or writing the file.
pub fn write_to(dir: &Path) -> io::Result<PathBuf> {
    parts().write_to(dir)
}

#[cfg(test)]
mod tests {
    use ui_settings::discover_parts;

    use super::{SHELL_SELECTORS, write_to};

    #[test]
    fn every_root_carries_the_shells_prefix() {
        assert_eq!(SHELL_SELECTORS.misnamed_roots(), Vec::<&str>::new());
    }

    /// The file lands in the directory it was given (a scratch one) and the reader Settings uses
    /// gives back every class the table publishes.
    #[test]
    fn the_written_file_is_read_back_by_the_parts_reader() {
        let dir = std::env::temp_dir().join(format!("shell-selectors-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let written = write_to(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        assert_eq!(written, dir.join("shell.toml"));
        let found = discover_parts(std::slice::from_ref(&dir));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].app, "shell");
        assert_eq!(found[0].classes, SHELL_SELECTORS.classes());
        assert!(
            found[0]
                .classes
                .iter()
                .any(|class| class == "shell-bar-group")
        );
    }
}
