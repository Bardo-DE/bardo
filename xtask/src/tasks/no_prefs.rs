//! `cargo xtask check no-prefs`: no identifier ends in `Prefs`. What the person changes is
//! `Settings`, what a packager or the process fixes at start is `Config`, and the arguments of
//! one call are `Options`.

use std::fs;

use anyhow::{Context, Result};

use super::{finish, git_lines};
use crate::workspace::Workspace;

/// This file shows the word in its tests, so its own text is not read.
const THIS_FILE: &str = "xtask/src/tasks/no_prefs.rs";

/// The identifiers of `line` that end in `Prefs`.
fn prefs_names(line: &str) -> Vec<&str> {
    line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|word| word.ends_with("Prefs"))
        .collect()
}

/// Checks every tracked `.rs` file under `crates/` and `xtask/`.
pub fn run() -> Result<bool> {
    let ws = Workspace::load()?;
    let files =
        git_lines(&ws.root, &["ls-files", "crates", "xtask"])?.context("git ls-files failed")?;
    let mut green = true;
    for file in files.iter().filter(|file| {
        file.as_str() != THIS_FILE
            && std::path::Path::new(file.as_str())
                .extension()
                .is_some_and(|ext| ext == "rs")
    }) {
        let text =
            fs::read_to_string(ws.root.join(file)).with_context(|| format!("reading {file}"))?;
        for (number, line) in text.lines().enumerate() {
            for name in prefs_names(line) {
                eprintln!(
                    "no-prefs: {file}:{}: `{name}`: say Settings, Config or Options",
                    number + 1
                );
                green = false;
            }
        }
    }
    Ok(finish("check no-prefs", green))
}

#[cfg(test)]
mod tests {
    use super::prefs_names;

    #[test]
    fn an_identifier_ending_in_prefs_is_a_hit() {
        assert_eq!(prefs_names("struct FolderPrefs {"), ["FolderPrefs"]);
        assert_eq!(
            prefs_names("let a: NoPrefs = TomlPrefs::new();"),
            ["NoPrefs", "TomlPrefs"]
        );
        assert_eq!(prefs_names("Prefs"), ["Prefs"]);
    }

    #[test]
    fn other_words_are_not_hits() {
        assert!(prefs_names("let prefs = 1; PrefsLike; the_prefs").is_empty());
        assert!(prefs_names("Settings and Config").is_empty());
    }
}
