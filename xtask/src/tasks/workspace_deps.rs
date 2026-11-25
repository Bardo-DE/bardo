//! `cargo xtask check workspace-deps`: every outside dependency is pinned once, in the root
//! `[workspace.dependencies]`. A member names it with `{ workspace = true }` and adds its own
//! features there; only path dependencies on the workspace's own crates are written in place.
//! A version or git revision written in a member drifts from the others and is missed when the
//! pin moves.
//!
//! A few crates are workspaces of their own on purpose (the UI kit's outside consumer, the
//! compositor's perf benches and its wlcs shim): the gate never builds them, so this check keeps
//! them sound. Such a manifest cannot inherit (`workspace = true` there names its own, empty
//! table), its lock file must still resolve, and a `[patch]` it repeats from the root must be the
//! root's.

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use toml::{Table, Value};

use super::{finish, git_lines};
use crate::workspace::Workspace;

/// `(member, dependency)` pairs that keep their own version until the keyboard stack moves to one
/// major (keyboard-types 0.8 and xkbcommon 0.9); the workspace table holds the version most
/// members use.
const UNTIL_KEYBOARD_UPGRADE: [(&str, &str); 3] = [
    ("keymap", "keyboard-types"),
    ("computer-use-seat-input", "xkbcommon"),
    ("fake-input", "xkbcommon"),
];

/// The dependency tables of a manifest: the three kinds at the top and under each `[target.*]`.
fn dependency_tables(manifest: &Table) -> Vec<&Table> {
    const KINDS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];
    let targets = manifest
        .get("target")
        .and_then(Value::as_table)
        .into_iter()
        .flat_map(Table::values)
        .filter_map(Value::as_table);
    std::iter::once(manifest)
        .chain(targets)
        .flat_map(|table| KINDS.iter().filter_map(|kind| table.get(*kind)?.as_table()))
        .collect()
}

/// The dependencies `manifest` pins itself: written as a version string, or as a table that is
/// neither `workspace = true` nor a `path`. Sorted, without repeats.
fn pinned_in_place(manifest: &Table) -> Vec<String> {
    let mut names: Vec<String> = dependency_tables(manifest)
        .into_iter()
        .flat_map(|table| table.iter())
        .filter(|(_, spec)| match spec {
            Value::Table(spec) => {
                spec.get("workspace").and_then(Value::as_bool) != Some(true)
                    && !spec.contains_key("path")
            }
            _ => true,
        })
        .map(|(name, _)| name.clone())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The dependencies `manifest` inherits with `workspace = true`. Sorted, without repeats.
fn inherited(manifest: &Table) -> Vec<String> {
    let mut names: Vec<String> = dependency_tables(manifest)
        .into_iter()
        .flat_map(|table| table.iter())
        .filter(|(_, spec)| {
            spec.as_table()
                .and_then(|spec| spec.get("workspace"))
                .and_then(Value::as_bool)
                == Some(true)
        })
        .map(|(name, _)| name.clone())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The `[patch.<source>]` entries of `nested` that the root also patches but differently, as
/// `source/name`. Sorted.
fn patches_unlike_root(root: &Table, nested: &Table) -> Vec<String> {
    let patch = |manifest: &Table| manifest.get("patch").and_then(Value::as_table).cloned();
    let (Some(root), Some(nested)) = (patch(root), patch(nested)) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for (source, entries) in &nested {
        let (Some(entries), Some(root_entries)) = (
            entries.as_table(),
            root.get(source).and_then(Value::as_table),
        ) else {
            continue;
        };
        for (name, spec) in entries {
            if root_entries
                .get(name)
                .is_some_and(|root_spec| root_spec != spec)
            {
                found.push(format!("{source}/{name}"));
            }
        }
    }
    found.sort();
    found
}

/// The workspaces of their own under `root`: every tracked `Cargo.toml` other than the root's
/// that has a `[workspace]` table, as its path relative to `root` and its parsed manifest.
pub fn nested_manifests(root: &Path) -> Result<Vec<(String, Table)>> {
    let files =
        git_lines(root, &["ls-files", "--", "*Cargo.toml"])?.context("git ls-files failed")?;
    let mut out = Vec::new();
    for file in files.into_iter().filter(|f| f.as_str() != "Cargo.toml") {
        let path = root.join(&file);
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let manifest: Table =
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        if manifest.contains_key("workspace") {
            out.push((file, manifest));
        }
    }
    Ok(out)
}

/// The problems of the workspaces of their own under `root`, as printable lines. (The gate also
/// compiles them; this part only reads and resolves.)
fn nested_problems(root: &Path, root_manifest: &Table) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for (file, manifest) in nested_manifests(root)? {
        for name in inherited(&manifest) {
            out.push(format!(
                "{file}: `{name}` says `workspace = true`, but this crate is a workspace of its \
                 own: write the version (or make it a member of the root workspace)"
            ));
        }
        for name in patches_unlike_root(root_manifest, &manifest) {
            out.push(format!(
                "{file}: [patch] `{name}` is not the root's; copy the root's entry"
            ));
        }
        let resolved = Command::new(env!("CARGO"))
            .args([
                "metadata",
                "--locked",
                "--format-version",
                "1",
                "--manifest-path",
            ])
            .arg(root.join(&file))
            .output()
            .context("could not start cargo metadata")?;
        if !resolved.status.success() {
            out.push(format!(
                "{file}: does not resolve with its lock file: {}",
                String::from_utf8_lossy(&resolved.stderr).trim()
            ));
        }
    }
    Ok(out)
}

/// Checks every workspace member's manifest, then the workspaces of their own.
pub fn run() -> Result<bool> {
    let ws = Workspace::load()?;
    let mut green = true;
    for (member, rel) in &ws.dirs {
        let path = ws.root.join(rel).join("Cargo.toml");
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let manifest: Table =
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        for dependency in pinned_in_place(&manifest) {
            if UNTIL_KEYBOARD_UPGRADE.contains(&(member.as_str(), dependency.as_str())) {
                continue;
            }
            eprintln!(
                "{}: workspace-deps: `{dependency}` is pinned here; add it to the root \
                 [workspace.dependencies] and write `{{ workspace = true }}` ({member})",
                path.display()
            );
            green = false;
        }
    }
    let root_path = ws.root.join("Cargo.toml");
    let root_manifest: Table = toml::from_str(
        &fs::read_to_string(&root_path)
            .with_context(|| format!("reading {}", root_path.display()))?,
    )
    .with_context(|| format!("parsing {}", root_path.display()))?;
    for problem in nested_problems(&ws.root, &root_manifest)? {
        eprintln!("workspace-deps: {problem}");
        green = false;
    }
    Ok(finish("check workspace-deps", green))
}

#[cfg(test)]
mod tests {
    use super::{inherited, patches_unlike_root, pinned_in_place};

    #[test]
    fn versions_and_git_pins_are_found_and_workspace_and_path_entries_are_not() {
        let manifest = toml::from_str(
            r#"
            [dependencies]
            anyhow = { workspace = true }
            tokio = { workspace = true, features = ["rt"] }
            own = { path = "../own" }
            clap = "4"

            [dev-dependencies]
            smithay = { git = "https://example.org/smithay", rev = "abc" }

            [target.'cfg(unix)'.build-dependencies]
            libc = { version = "0.2", default-features = false }
            "#,
        )
        .expect("the manifest parses");
        assert_eq!(pinned_in_place(&manifest), ["clap", "libc", "smithay"]);
    }

    #[test]
    fn inherited_entries_are_found() {
        let manifest = toml::from_str(
            r#"
            [dependencies]
            anyhow = { workspace = true }
            clap = "4"
            [dev-dependencies]
            serde = { workspace = true, features = ["derive"] }
            "#,
        )
        .expect("the manifest parses");
        assert_eq!(inherited(&manifest), ["anyhow", "serde"]);
    }

    #[test]
    fn a_repeated_patch_must_be_the_roots_and_one_the_root_lacks_is_free() {
        let root = toml::from_str(
            r#"
            [patch.crates-io]
            vello = { git = "https://example.org/vello", rev = "a" }
            only-root = { path = "vendor/x" }
            "#,
        )
        .expect("root parses");
        let nested = toml::from_str(
            r#"
            [patch.crates-io]
            vello = { git = "https://example.org/vello", rev = "b" }
            only-here = { path = "x" }
            "#,
        )
        .expect("nested parses");
        assert_eq!(patches_unlike_root(&root, &nested), ["crates-io/vello"]);
        assert!(patches_unlike_root(&root, &root).is_empty());
    }
}
