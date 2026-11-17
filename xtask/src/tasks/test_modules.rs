//! `cargo xtask check test-modules`: every file of a crate's `tests/it` directory is a module of
//! its `main.rs`. A file `main.rs` does not declare is never compiled, so its tests never run and
//! nothing says so.

use std::fs;
use std::path::Path;

use anyhow::Result;

use super::finish;
use crate::workspace::Workspace;

/// The module `line` declares, if it declares one: `mod name;`, `pub mod name;`, `mod name {`.
fn declared_by(line: &str) -> Option<&str> {
    let rest = line.trim_start();
    let rest = rest.strip_prefix("pub(crate) ").unwrap_or(rest);
    let rest = rest.strip_prefix("pub ").unwrap_or(rest);
    let rest = rest.strip_prefix("mod ")?;
    rest.split([';', ' ', '{']).next()
}

/// The names in `entries` (file names) that `main` leaves out: a `name.rs` that no `mod name`
/// declares. A directory next to it holds the children of `name`, or data, and is not a module.
fn undeclared(main: &str, entries: &[String]) -> Vec<String> {
    let declared: Vec<&str> = main.lines().filter_map(declared_by).collect();
    let mut left_out: Vec<String> = entries
        .iter()
        .filter(|name| {
            name.strip_suffix(".rs")
                .filter(|stem| *stem != "main")
                .is_some_and(|module| !declared.contains(&module))
        })
        .cloned()
        .collect();
    left_out.sort();
    left_out
}

/// The `.rs` files of `dir`.
#[expect(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "Rust source files are named in lower case; a case-insensitive match would admit files cargo does not read"
)]
fn entries_of(dir: &Path) -> Result<Vec<String>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !path.is_dir() && name.ends_with(".rs") {
            entries.push(name.to_owned());
        }
    }
    Ok(entries)
}

/// Checks every workspace member that has a `tests/it/main.rs`.
pub fn run() -> Result<bool> {
    let ws = Workspace::load()?;
    let mut green = true;
    for (name, rel) in &ws.dirs {
        let dir = ws.root.join(rel).join("tests/it");
        let Ok(main) = fs::read_to_string(dir.join("main.rs")) else {
            continue;
        };
        for left_out in undeclared(&main, &entries_of(&dir)?) {
            eprintln!(
                "{}: test-module: `{left_out}` is not a module of main.rs, so it never runs ({name})",
                dir.join(&left_out).display()
            );
            green = false;
        }
    }
    Ok(finish("check test-modules", green))
}

#[cfg(test)]
mod tests {
    use super::undeclared;

    fn names(files: &[&str]) -> Vec<String> {
        files.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn a_file_without_a_mod_line_is_left_out() {
        let main = "mod alpha;\npub mod beta;\n#[path = \"x/y.rs\"]\nmod gamma;\nmod delta {\n}\n";
        let entries = names(&[
            "main.rs", "alpha.rs", "beta.rs", "gamma.rs", "delta.rs", "lost.rs",
        ]);
        assert_eq!(undeclared(main, &entries), ["lost.rs"]);
    }
}
