//! `cargo xtask check no-mod-rs`: a module `foo` with children is `foo.rs` beside a `foo/`
//! directory, never `foo/mod.rs`, and a module is found by its place, never by a `#[path]`
//! attribute.

use std::fs;

use anyhow::{Context, Result};

use super::{finish, git_lines};
use crate::workspace::Workspace;

/// This file shows the pattern in its tests, so its own text is not read.
const THIS_FILE: &str = "xtask/src/tasks/no_mod_rs.rs";

/// Whether the tracked file `path` is a `mod.rs`.
fn is_mod_rs(path: &str) -> bool {
    path == "mod.rs" || path.ends_with("/mod.rs")
}

/// Whether `line` is a `#[path = "..."]` attribute (also inside `cfg_attr`).
fn is_path_attribute(line: &str) -> bool {
    let line = line.trim_start();
    if line.starts_with("//") {
        return false;
    }
    let attribute = line
        .strip_prefix("#[")
        .or_else(|| line.strip_prefix("#!["))
        .map(str::trim_start);
    attribute.is_some_and(|rest| {
        rest.strip_prefix("path")
            .is_some_and(|after| after.trim_start().starts_with('='))
            || (rest.starts_with("cfg_attr(") && rest.contains("path ="))
    })
}

/// Checks every tracked file under `crates/` and `xtask/`.
pub fn run() -> Result<bool> {
    let ws = Workspace::load()?;
    let files =
        git_lines(&ws.root, &["ls-files", "crates", "xtask"])?.context("git ls-files failed")?;
    let mut green = true;
    for file in &files {
        if is_mod_rs(file) {
            eprintln!("no-mod-rs: {file}: name it after its directory (`foo/mod.rs` is `foo.rs`)");
            green = false;
        }
        let rust = std::path::Path::new(file)
            .extension()
            .is_some_and(|ext| ext == "rs");
        if !rust || file == THIS_FILE {
            continue;
        }
        let text =
            fs::read_to_string(ws.root.join(file)).with_context(|| format!("reading {file}"))?;
        for (number, line) in text.lines().enumerate() {
            if is_path_attribute(line) {
                eprintln!(
                    "no-mod-rs: {file}:{}: `#[path]` names a module by file; put the file where `mod` looks",
                    number + 1
                );
                green = false;
            }
        }
    }
    Ok(finish("check no-mod-rs", green))
}

#[cfg(test)]
mod tests {
    use super::{is_mod_rs, is_path_attribute};

    #[test]
    fn a_mod_rs_file_is_a_hit() {
        assert!(is_mod_rs("crates/a/b/src/foo/mod.rs"));
        assert!(is_mod_rs("mod.rs"));
        assert!(!is_mod_rs("crates/a/b/src/foo.rs"));
        assert!(!is_mod_rs("crates/a/b/src/foo/module.rs"));
        assert!(!is_mod_rs("crates/a/b/src/foo/not_mod.rs"));
    }

    #[test]
    fn a_path_attribute_is_a_hit() {
        assert!(is_path_attribute("#[path = \"x/y.rs\"]"));
        assert!(is_path_attribute("    #[path=\"x.rs\"]"));
        assert!(is_path_attribute("#[cfg_attr(test, path = \"t.rs\")]"));
        assert!(!is_path_attribute("//! `#[path = \"x.rs\"] mod x;`"));
        assert!(!is_path_attribute("#[cfg(test)]"));
        assert!(!is_path_attribute("#[pathological]"));
    }
}
