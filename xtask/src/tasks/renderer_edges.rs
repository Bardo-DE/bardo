//! `cargo xtask check renderer-edges`: only the renderer crates and the programs depend on the
//! renderer. A UI library compiles dioxus and the design system, not Blitz and wgpu, so it takes
//! the seams (`layer-shell-types`, `layer-shell-hooks`, `ui-host-hooks`) and leaves `ui-blitz`,
//! `layer-shell-host`, the Blitz crates and wgpu to the crates that run a window: the renderer
//! fills the seams, and a program picks the renderer. Test harnesses reach it from
//! `[dev-dependencies]`, which this check does not read.

use std::fs;

use anyhow::{Context, Result};
use toml::{Table, Value};

use super::finish;
use crate::workspace::Workspace;

/// The crates only an allowlisted member may name in `[dependencies]` (or a target's).
const RENDERER: [&str; 6] = [
    "ui-blitz",
    "layer-shell-host",
    "blitz-dom",
    "blitz-traits",
    "blitz-embed",
    "wgpu",
];

/// The members that may name them: the renderer crates, the crates that run one, and the programs.
const ALLOWED: [&str; 12] = [
    // The renderer for windows: `launch`, the window loop and the Blitz document wiring.
    "ui-blitz",
    // The renderer for layer-shell surfaces: the Wayland host and its GPU device.
    "layer-shell-host",
    // The renderers' unsafe seam: its `wgpu` feature makes the wgpu surface over a Wayland surface.
    "wayland-raw-handle",
    // The Blitz repairs both renderers share, written against blitz-dom and wgpu.
    "blitz-embed",
    // The test driver: builds a document from the renderer's own parts, so a test runs what a window runs.
    "ui-harness",
    // Hands the layer-shell host its configuration: the faces, the pre-paint pass, the file roots.
    "shell-host-config",
    // Its `pane` feature draws PDF thumbnails and texture layers through the renderer's GPU seam.
    "anyview-peek",
    // The program that runs the shell: it picks the host and the window renderer.
    "shell-app",
    // The toolkit's example of an outside app: an app picks the window renderer, as any app does.
    "consumer",
    // The program of the file manager: it opens the window.
    "files",
    // The program of the Settings app: it opens the window and serves the app.
    "settings-app",
    // The gallery program: it opens a window to show every component.
    "ui-gallery",
];

/// The dependency tables that build a crate for its users: `[dependencies]` and each target's.
/// Dev and build dependencies are not read.
fn normal_tables(manifest: &Table) -> Vec<&Table> {
    let targets = manifest
        .get("target")
        .and_then(Value::as_table)
        .into_iter()
        .flat_map(Table::values)
        .filter_map(Value::as_table);
    std::iter::once(manifest)
        .chain(targets)
        .filter_map(|table| table.get("dependencies")?.as_table())
        .collect()
}

/// The renderer crates `manifest` names as dependencies (by the name they have in the registry,
/// whatever the key renames them to), sorted, without repeats.
fn renderer_dependencies(manifest: &Table) -> Vec<String> {
    let mut names: Vec<String> = normal_tables(manifest)
        .into_iter()
        .flat_map(|table| table.iter())
        .map(|(key, spec)| {
            spec.as_table()
                .and_then(|spec| spec.get("package"))
                .and_then(Value::as_str)
                .unwrap_or(key)
                .to_owned()
        })
        .filter(|name| RENDERER.contains(&name.as_str()))
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
        if ALLOWED.contains(&member.as_str()) {
            continue;
        }
        let path = ws.root.join(rel).join("Cargo.toml");
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let manifest: Table =
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        for dependency in renderer_dependencies(&manifest) {
            eprintln!(
                "{}: renderer-edges: `{dependency}` is a renderer crate; a library takes the \
                 seam instead (layer-shell-types, layer-shell-hooks or ui-host-hooks), and a \
                 test harness lists it under [dev-dependencies] ({member})",
                path.display()
            );
            green = false;
        }
    }
    Ok(finish("check renderer-edges", green))
}

#[cfg(test)]
mod tests {
    use super::{ALLOWED, RENDERER, renderer_dependencies};

    #[test]
    fn a_renderer_crate_is_found_in_dependencies_and_a_target_table_but_not_in_dev_tables() {
        let manifest = toml::from_str(
            r#"
            [dependencies]
            dioxus = { workspace = true }
            ui-blitz = { path = "../ui-blitz" }
            blitz-dom = { workspace = true }

            [dev-dependencies]
            layer-shell-host = { path = "../layer-shell-host" }
            ui-harness = { path = "../ui-harness" }

            [build-dependencies]
            wgpu = { workspace = true }

            [target.'cfg(unix)'.dependencies]
            wgpu = { workspace = true }
            "#,
        )
        .expect("the manifest parses");
        assert_eq!(
            renderer_dependencies(&manifest),
            ["blitz-dom", "ui-blitz", "wgpu"]
        );
    }

    #[test]
    fn a_renamed_dependency_is_read_by_the_crate_it_names() {
        let manifest = toml::from_str(
            r#"
            [dependencies]
            renderer = { package = "ui-blitz", path = "../ui-blitz" }
            ui-blitz-docs = { path = "../docs" }
            "#,
        )
        .expect("the manifest parses");
        assert_eq!(renderer_dependencies(&manifest), ["ui-blitz"]);
    }

    #[test]
    fn no_seam_crate_is_on_the_allowlist() {
        for seam in ["layer-shell-types", "layer-shell-hooks", "ui-host-hooks"] {
            assert!(!ALLOWED.contains(&seam), "{seam} is a seam, not a renderer");
            assert!(
                !RENDERER.contains(&seam),
                "{seam} is a seam, not a renderer"
            );
        }
    }
}
