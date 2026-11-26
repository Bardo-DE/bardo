//! `cargo xtask check lints`: every crate takes the workspace lints, and the two ways out of them are
//! the ones its `layers.toml` row names.
//!
//! - A row with `unsafe = true` marks one of the small crates that hold our unsafe code
//!   (`UNSAFE.md`). Only those crates opt out of `unsafe_code`, and their own `[lints]` table is the
//!   workspace table with `unsafe_code = "deny"` in place of `"forbid"` (Cargo cannot override one
//!   entry of an inherited table). Every other crate has `[lints] workspace = true`.
//! - A row with `lints = "baseline"` marks a crate that does not document all its public items yet:
//!   it carries `#![expect(missing_docs, reason = "...")]` at its target roots until its domain is
//!   refactored, then the row flips to `"full"` (the default). The `expect` fails once the docs are
//!   complete, so a crate cannot stay baseline by accident; this check keeps the opt-out to the
//!   baseline crates and drops the row once no opt-out is left.
//!
//! Generated code and macro output may `allow(missing_docs, reason)` anywhere, as every `allow` is
//! for them; tests, examples and benches may also `expect` it.

use std::collections::BTreeMap;
use std::fs;

use anyhow::{Context, Result};
use serde::Deserialize;

use super::{finish, git_lines};
use crate::workspace::Workspace;

/// The rules file, at the workspace root.
const FILE: &str = "layers.toml";

/// The part of `layers.toml` this check reads: the crate rows' two lint flags.
#[derive(Debug, Default, Deserialize)]
struct Rows {
    #[serde(default, rename = "crate")]
    crates: BTreeMap<String, Flags>,
}

/// A crate row's lint flags.
#[derive(Debug, Default, Deserialize)]
struct Flags {
    #[serde(default)]
    lints: Level,
    #[serde(default, rename = "unsafe")]
    is_unsafe: bool,
}

/// How much of the workspace lint set a crate meets.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Level {
    /// Everything, `missing_docs` included.
    #[default]
    Full,
    /// Everything but `missing_docs`.
    Baseline,
}

/// The lint opt-outs one source file holds.
#[derive(Debug, Default, PartialEq, Eq)]
struct OptOuts {
    /// Lines of `allow`/`expect` attributes naming `unsafe_code`.
    unsafe_code: Vec<usize>,
    /// Lines of `expect` attributes naming `missing_docs`: the baseline opt-out. An `allow` of it
    /// is for generated code and macro output, as every `allow` is.
    missing_docs: Vec<usize>,
}

/// The `allow` and `expect` attributes of `text` that name `unsafe_code`, and the `expect`
/// attributes that name `missing_docs`, by the line the attribute starts on. An attribute may span
/// lines; comments and the code after the first `#[cfg(test)]` are not read.
fn opt_outs(text: &str) -> OptOuts {
    let mut found = OptOuts::default();
    let mut attribute: Option<(usize, String)> = None;
    for (index, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("#[cfg(test)]") {
            // Test code follows: an opt-out there is for the tests, and their text may quote one.
            break;
        }
        let code = line.split("//").next().unwrap_or_default().trim();
        if attribute.is_none() {
            let Some(rest) = ["#![allow(", "#![expect(", "#[allow(", "#[expect("]
                .iter()
                .find_map(|start| code.strip_prefix(start))
            else {
                continue;
            };
            let expect = code.starts_with("#![expect(") || code.starts_with("#[expect(");
            attribute = Some((
                index + 1,
                format!("{}{rest}", if expect { "!" } else { "" }),
            ));
        } else if let Some((_, body)) = attribute.as_mut() {
            body.push(' ');
            body.push_str(code);
        }
        if let Some((start, body)) = attribute.take_if(|(_, body)| body.contains(")]")) {
            let (expect, body) = match body.strip_prefix('!') {
                Some(body) => (true, body),
                None => (false, body.as_str()),
            };
            let names = body.split(")]").next().unwrap_or_default();
            let names: Vec<&str> = names
                .split(',')
                .map(str::trim)
                .take_while(|name| !name.starts_with("reason"))
                .collect();
            if names.contains(&"unsafe_code") {
                found.unsafe_code.push(start);
            }
            if expect && names.contains(&"missing_docs") {
                found.missing_docs.push(start);
            }
        }
    }
    found
}

/// Whether `file` (relative to its crate) is test, example or bench code.
fn is_test_code(file: &str) -> bool {
    ["tests/", "examples/", "benches/"]
        .iter()
        .any(|dir| file.starts_with(dir) || file.contains(&format!("/{dir}")))
}

/// The findings for one crate's `[lints]` table against the workspace's.
fn table_findings(
    name: &str,
    manifest: &str,
    flags: &Flags,
    lints: Option<&toml::Value>,
    workspace: &toml::Value,
) -> Vec<String> {
    let inherits = lints
        .and_then(|lints| lints.get("workspace"))
        .and_then(toml::Value::as_bool)
        == Some(true);
    if inherits && lints.and_then(toml::Value::as_table).map(toml::Table::len) == Some(1) {
        return Vec::new();
    }
    if !flags.is_unsafe {
        return vec![format!(
            "{manifest}: {name} must take the workspace lints: `[lints] workspace = true` and nothing else"
        )];
    }
    let mut expected = workspace.clone();
    if let Some(rust) = expected.get_mut("rust").and_then(toml::Value::as_table_mut) {
        rust.insert("unsafe_code".to_owned(), toml::Value::from("deny"));
    }
    if lints == Some(&expected) {
        Vec::new()
    } else {
        vec![format!(
            "{manifest}: {name} is an unsafe crate: its [lints] table is the workspace's with `unsafe_code = \"deny\"`, nothing else changed"
        )]
    }
}

/// Checks every member's `[lints]` table and its `unsafe_code` and `missing_docs` opt-outs.
pub fn run() -> Result<bool> {
    let ws = Workspace::load()?;
    let rows: Rows = toml::from_str(
        &fs::read_to_string(ws.root.join(FILE))
            .with_context(|| format!("could not read {FILE} at the workspace root"))?,
    )
    .with_context(|| format!("could not parse {FILE}"))?;
    let root: toml::Table = toml::from_str(
        &fs::read_to_string(ws.root.join("Cargo.toml")).context("could not read Cargo.toml")?,
    )
    .context("could not parse Cargo.toml")?;
    let workspace_lints = root
        .get("workspace")
        .and_then(|workspace| workspace.get("lints"))
        .context("Cargo.toml has no [workspace.lints]")?;

    let mut findings = Vec::new();
    for (name, dir) in &ws.dirs {
        if name == "xtask" && !rows.crates.contains_key(name) {
            continue;
        }
        let default = Flags::default();
        let flags = rows.crates.get(name).unwrap_or(&default);
        let manifest = dir.join("Cargo.toml");
        let manifest_text = manifest.display().to_string();
        let table: toml::Table = toml::from_str(
            &fs::read_to_string(ws.root.join(&manifest))
                .with_context(|| format!("reading {manifest_text}"))?,
        )
        .with_context(|| format!("parsing {manifest_text}"))?;
        findings.extend(table_findings(
            name,
            &manifest_text,
            flags,
            table.get("lints"),
            workspace_lints,
        ));

        let files = git_lines(&ws.root.join(dir), &["ls-files", "--", "*.rs"])?
            .with_context(|| format!("git ls-files failed in {}", dir.display()))?;
        let (mut unsafe_seen, mut baseline_seen) = (false, false);
        for file in &files {
            let path = dir.join(file);
            let text = fs::read_to_string(ws.root.join(&path))
                .with_context(|| format!("reading {}", path.display()))?;
            let found = opt_outs(&text);
            for line in &found.unsafe_code {
                unsafe_seen = true;
                if !flags.is_unsafe {
                    findings.push(format!(
                        "{}:{line}: {name} opts out of unsafe_code, but its {FILE} row is not `unsafe = true`: unsafe code goes in the unsafe crates (UNSAFE.md)",
                        path.display()
                    ));
                }
            }
            if is_test_code(file) {
                continue;
            }
            for line in &found.missing_docs {
                baseline_seen = true;
                if flags.lints != Level::Baseline {
                    findings.push(format!(
                        "{}:{line}: {name} opts out of missing_docs, but its {FILE} row is not `lints = \"baseline\"`",
                        path.display()
                    ));
                }
            }
        }
        if flags.is_unsafe && !unsafe_seen {
            findings.push(format!(
                "{FILE}: {name} has no unsafe code left: drop `unsafe = true` and take the workspace lints"
            ));
        }
        if flags.lints == Level::Baseline && !baseline_seen {
            findings.push(format!(
                "{FILE}: {name} documents everything now: drop `lints = \"baseline\"`"
            ));
        }
    }
    findings.sort();
    for finding in &findings {
        eprintln!("lints: {finding}");
    }
    Ok(finish("check lints", findings.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opt_outs_are_found_by_name_and_line_across_lines() {
        let text = "//! doc\n\
                    #![expect(missing_docs, reason = \"baseline\")]\n\
                    #[allow(missing_docs, reason = \"generated\")]\n\
                    #![expect(\n    unsafe_code,\n    reason = \"ffi\"\n)]\n\
                    #[allow(dead_code, reason = \"unsafe_code is only named in the reason\")]\n\
                    // #[allow(unsafe_code)] in a comment\n\
                    fn f() {}\n";
        assert_eq!(
            opt_outs(text),
            OptOuts {
                unsafe_code: vec![4],
                missing_docs: vec![2],
            }
        );
    }

    #[test]
    fn test_code_after_cfg_test_is_not_read() {
        let text = "fn f() {}\n#[cfg(test)]\nmod tests {\n    #![expect(missing_docs)]\n}\n";
        assert_eq!(opt_outs(text), OptOuts::default());
    }

    #[test]
    fn test_code_is_told_apart_from_src() {
        assert!(is_test_code("tests/it/main.rs"));
        assert!(is_test_code("examples/agent.rs"));
        assert!(!is_test_code("src/tests_of_nothing.rs"));
        assert!(!is_test_code("src/lib.rs"));
    }

    fn workspace() -> toml::Value {
        toml::from_str("[rust]\nunsafe_code = \"forbid\"\nmissing_docs = \"warn\"\n[clippy]\npedantic = \"warn\"\n")
            .expect("parses")
    }

    #[test]
    fn only_unsafe_crates_have_their_own_table_and_it_differs_in_one_entry() {
        let inherit: toml::Value = toml::from_str("workspace = true").expect("parses");
        let own: toml::Value = toml::from_str(
            "[rust]\nunsafe_code = \"deny\"\nmissing_docs = \"warn\"\n[clippy]\npedantic = \"warn\"\n",
        )
        .expect("parses");
        let drifted: toml::Value =
            toml::from_str("[rust]\nunsafe_code = \"deny\"\n[clippy]\npedantic = \"warn\"\n")
                .expect("parses");
        let safe = Flags::default();
        let marked = Flags {
            is_unsafe: true,
            ..Flags::default()
        };
        let ws = workspace();
        assert!(table_findings("a", "a/Cargo.toml", &safe, Some(&inherit), &ws).is_empty());
        assert_eq!(
            table_findings("a", "a/Cargo.toml", &safe, None, &ws).len(),
            1
        );
        assert_eq!(
            table_findings("a", "a/Cargo.toml", &safe, Some(&own), &ws).len(),
            1
        );
        assert!(table_findings("u", "u/Cargo.toml", &marked, Some(&own), &ws).is_empty());
        assert!(table_findings("u", "u/Cargo.toml", &marked, Some(&inherit), &ws).is_empty());
        assert_eq!(
            table_findings("u", "u/Cargo.toml", &marked, Some(&drifted), &ws).len(),
            1
        );
    }
}
