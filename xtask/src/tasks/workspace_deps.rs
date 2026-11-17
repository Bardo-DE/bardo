//! `cargo xtask check workspace-deps`: every outside dependency is pinned once, in the root
//! `[workspace.dependencies]`. A member names it with `{ workspace = true }` and adds its own
//! features there; only path dependencies on the workspace's own crates are written in place.
//! A version or git revision written in a member drifts from the others and is missed when the
//! pin moves.

use std::fs;

use anyhow::{Context, Result};
use toml::{Table, Value};

use super::finish;
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

/// Checks every workspace member's manifest.
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
    Ok(finish("check workspace-deps", green))
}

#[cfg(test)]
mod tests {
    use super::pinned_in_place;

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
}
