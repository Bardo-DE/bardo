//! `cargo xtask check test-modules`: the layout of every crate's integration tests.
//!
//! - Every file of `tests/it` is a module of its `main.rs`. A file `main.rs` does not declare is
//!   never compiled, so its tests never run and nothing says so.
//! - A crate has one integration test binary, `tests/it/main.rs`. Any other `tests/*.rs` file,
//!   `tests/<dir>/main.rs` or `[[test]]` entry is a second binary, which links the crate's
//!   dependencies again and is a place for tests to hide from a plain look at `tests/it`. The few
//!   crates that need one are in [`SECOND_BINARIES`], each with its reason.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use toml::{Table, Value};

use super::finish;
use crate::workspace::Workspace;

/// The one integration test binary every crate has.
const ONE_BINARY: &str = "it";

/// The second test binaries that stay: `(package, binary, why it cannot be a module of `it`)`.
const SECOND_BINARIES: [(&str, &str, &str); 6] = [
    (
        "ui-blitz",
        "pdf_thumb",
        "shares the process-wide PDF worker queue and its cache with `pdf_thumb_queue` and would count its rasters",
    ),
    (
        "ui-blitz",
        "pdf_thumb_queue",
        "counts the process-wide rasters of the PDF worker, which no other test may add to",
    ),
    (
        "ui-blitz",
        "spell_worker",
        "starts the process-wide spell service on its own dictionary",
    ),
    (
        "ui-blitz",
        "spell_edit",
        "starts the process-wide spell service on its own dictionary",
    ),
    (
        "anyview-archive",
        "bounded_streams",
        "resets the kernel's high-water mark of the process's memory, so no other test may run beside it",
    ),
    (
        "anyview-text",
        "peek_memory",
        "resets the kernel's high-water mark of the process's memory, so no other test may run beside it",
    ),
];

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

/// The test binaries a crate has: every `tests/*.rs` file and `tests/<dir>/main.rs` cargo finds
/// by itself (named for the file or the directory), and every `[[test]]` entry of `manifest`.
fn test_binaries(tests: &Path, manifest: &str) -> Result<BTreeSet<String>> {
    let mut binaries = BTreeSet::new();
    if let Ok(entries) = fs::read_dir(tests) {
        for entry in entries {
            let path = entry?.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if path.is_dir() {
                if path.join("main.rs").is_file() {
                    binaries.insert(name.to_owned());
                }
            } else if let Some(stem) = name.strip_suffix(".rs") {
                binaries.insert(stem.to_owned());
            }
        }
    }
    let manifest: Table = toml::from_str(manifest).context("parsing the manifest")?;
    let declared = manifest.get("test").and_then(Value::as_array);
    for entry in declared.into_iter().flatten() {
        if let Some(name) = entry.get("name").and_then(Value::as_str) {
            binaries.insert(name.to_owned());
        }
    }
    Ok(binaries)
}

/// The binaries in `binaries` that `package` may not have: all but `it` and its own entries of
/// [`SECOND_BINARIES`].
fn second_binaries(package: &str, binaries: &BTreeSet<String>) -> Vec<String> {
    binaries
        .iter()
        .filter(|binary| binary.as_str() != ONE_BINARY)
        .filter(|binary| {
            !SECOND_BINARIES
                .iter()
                .any(|(owner, name, _)| *owner == package && name == binary)
        })
        .cloned()
        .collect()
}

/// Checks every workspace member: one integration test binary, and every file of its `tests/it`
/// is a module of `main.rs`.
pub fn run() -> Result<bool> {
    let ws = Workspace::load()?;
    let mut green = true;
    for (name, rel) in &ws.dirs {
        let crate_dir = ws.root.join(rel);
        let manifest = fs::read_to_string(crate_dir.join("Cargo.toml"))
            .with_context(|| format!("reading the manifest of {name}"))?;
        let binaries = test_binaries(&crate_dir.join("tests"), &manifest)
            .with_context(|| format!("reading the tests of {name}"))?;
        for binary in second_binaries(name, &binaries) {
            eprintln!(
                "{}: test-layout: `{binary}` is a second test binary; make it a module of tests/it/main.rs ({name})",
                crate_dir.join("tests").display()
            );
            green = false;
        }
        let dir = crate_dir.join("tests/it");
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
    use std::collections::BTreeSet;

    use super::{second_binaries, test_binaries, undeclared};

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

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn only_it_is_a_test_binary_without_a_reason() {
        let found = set(&["it", "extra", "other"]);
        assert_eq!(second_binaries("some-crate", &found), ["extra", "other"]);
        assert!(second_binaries("some-crate", &set(&["it"])).is_empty());
    }

    #[test]
    fn a_listed_second_binary_is_allowed_only_for_its_crate() {
        let found = set(&["it", "spell_edit"]);
        assert!(second_binaries("ui-blitz", &found).is_empty());
        assert_eq!(second_binaries("other-crate", &found), ["spell_edit"]);
    }

    #[test]
    fn a_manifest_entry_is_a_binary_too() {
        let dir = std::env::temp_dir().join(format!("xtask-test-layout-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("by_dir")).unwrap();
        std::fs::write(dir.join("by_dir/main.rs"), "").unwrap();
        std::fs::write(dir.join("by_file.rs"), "").unwrap();
        std::fs::create_dir_all(dir.join("data")).unwrap();
        let manifest = "[[test]]\nname = \"declared\"\npath = \"x.rs\"\n";
        let binaries = test_binaries(&dir, manifest).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(binaries, set(&["by_dir", "by_file", "declared"]));
    }
}
