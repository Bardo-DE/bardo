//! `cargo xtask check domain-order`: a library crate depends only on its own domain folder
//! (`crates/<domain>/`) or a lower one, so the domains stack instead of forming a cycle. Programs
//! (a `bin` target, or a name that says so) sit on top and may depend on any domain.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cargo_metadata::{DependencyKind, MetadataCommand};

use super::finish;
use crate::workspace::Workspace;

/// The domain folders under `crates/`, lowest first: a library may depend on its own domain and the ones before it.
const ORDER: [&str; 31] = [
    "base",
    "bus",
    "sandbox",
    "identity",
    "platform",
    "network",
    "wayland",
    "settings",
    "keymap",
    "blitz-embed",
    "plugin",
    "ui",
    "accounts",
    "provenance",
    "models",
    "engines",
    "inference",
    "memory",
    "actions",
    "speech",
    "compositor",
    "computer-use",
    "anyview",
    "input",
    "layer-shell",
    "sync",
    "portal",
    "updater",
    "files",
    "shell",
    "settings-app",
];

/// A crate name ending in one of these is a program, not a library.
const PROGRAM_SUFFIXES: [&str; 14] = [
    "-daemon", "-app", "-cli", "-bin", "-eval", "-ui", "-testkit", "-fake", "-accept", "-gallery",
    "-bench", "-demo", "-tool", "-harness",
];

/// A workspace package, as far as the order is concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Package {
    name: String,
    manifest: PathBuf,
    /// The `<domain>` of `crates/<domain>/...`.
    domain: String,
    program: bool,
}

/// The position of `domain` in [`ORDER`]; `None` for a domain the order does not name.
fn rank(domain: &str) -> Option<usize> {
    ORDER.iter().position(|known| *known == domain)
}

/// The domain of a package whose manifest directory is `dir` (relative to the root); `None` outside `crates/`.
fn domain_of(dir: &Path) -> Option<String> {
    let mut parts = dir.components();
    if parts.next()?.as_os_str() != "crates" {
        return None;
    }
    Some(parts.next()?.as_os_str().to_str()?.to_owned())
}

/// Whether a package is a program: it has a `bin` target, its name says so, or it lives under an `examples` folder.
fn is_program(name: &str, dir: &Path, has_bin: bool) -> bool {
    has_bin
        || PROGRAM_SUFFIXES.iter().any(|suffix| name.ends_with(suffix))
        || name.contains("-eval-")
        || dir.components().any(|part| part.as_os_str() == "examples")
}

/// The report lines for `edges` (`(dependent, dependency)` names): one for each library that
/// depends on a library of a higher domain, and one for each package in a domain with no place in the order.
fn violations(packages: &[Package], edges: &[(String, String)]) -> Vec<String> {
    let find = |name: &str| packages.iter().find(|package| package.name == name);
    let mut lines = Vec::new();
    for package in packages {
        if rank(&package.domain).is_none() {
            lines.push(format!(
                "{}:1: domain-order: {} is in domain `{}`, which the order does not name",
                package.manifest.display(),
                package.name,
                package.domain
            ));
        }
    }
    for (dependent, dependency) in edges {
        let (Some(from), Some(to)) = (find(dependent), find(dependency)) else {
            continue;
        };
        if from.program || to.program {
            continue;
        }
        if let (Some(from_rank), Some(to_rank)) = (rank(&from.domain), rank(&to.domain))
            && to_rank > from_rank
        {
            lines.push(format!(
                "{}:1: domain-order: {} ({}) -> {} ({})",
                from.manifest.display(),
                from.name,
                from.domain,
                to.name,
                to.domain
            ));
        }
    }
    lines.sort();
    lines.dedup();
    lines
}

/// Checks every workspace library's normal dependencies on other workspace libraries.
pub fn run() -> Result<bool> {
    let ws = Workspace::load()?;
    let meta = MetadataCommand::new()
        .no_deps()
        .exec()
        .context("cargo metadata failed")?;
    let mut packages = Vec::new();
    let mut edges = Vec::new();
    for package in &meta.packages {
        let name = package.name.to_string();
        let Some(dir) = ws.dirs.get(&name) else {
            continue;
        };
        let Some(domain) = domain_of(dir) else {
            continue;
        };
        let has_bin = package.targets.iter().any(cargo_metadata::Target::is_bin);
        packages.push(Package {
            program: is_program(&name, dir, has_bin),
            manifest: package.manifest_path.clone().into_std_path_buf(),
            domain,
            name: name.clone(),
        });
        for dependency in &package.dependencies {
            if dependency.kind == DependencyKind::Normal {
                edges.push((name.clone(), dependency.name.clone()));
            }
        }
    }
    let lines = violations(&packages, &edges);
    for line in &lines {
        eprintln!("{line}");
    }
    Ok(finish("check domain-order", lines.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::{ORDER, Package, domain_of, is_program, rank, violations};
    use std::path::{Path, PathBuf};

    fn package(name: &str, domain: &str, program: bool) -> Package {
        Package {
            name: name.to_owned(),
            manifest: PathBuf::from(format!("crates/{domain}/{name}/Cargo.toml")),
            domain: domain.to_owned(),
            program,
        }
    }

    fn edge(from: &str, to: &str) -> (String, String) {
        (from.to_owned(), to.to_owned())
    }

    #[test]
    fn the_order_names_each_domain_once() {
        let mut sorted = ORDER.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ORDER.len());
        assert!(rank("base") < rank("ui"));
        assert!(rank("ui") < rank("shell"));
        assert_eq!(rank("nowhere"), None);
    }

    #[test]
    fn a_domain_is_the_folder_under_crates() {
        assert_eq!(
            domain_of(Path::new("crates/ui/ui-core")).as_deref(),
            Some("ui")
        );
        assert_eq!(domain_of(Path::new("xtask")), None);
    }

    #[test]
    fn a_program_has_a_bin_a_telling_name_or_an_examples_folder() {
        let dir = Path::new("crates/ui/ui-core");
        assert!(is_program("ui-core", dir, true));
        assert!(is_program("account-daemon", dir, false));
        assert!(is_program("computer-use-eval-cloud", dir, false));
        assert!(is_program("x", Path::new("crates/ui/examples/x"), false));
        assert!(!is_program("ui-core", dir, false));
        assert!(!is_program("model-fakery", dir, false));
    }

    #[test]
    fn a_library_may_depend_on_its_own_domain_or_a_lower_one() {
        let packages = [
            package("a", "ui", false),
            package("b", "ui", false),
            package("c", "base", false),
        ];
        let edges = [edge("a", "b"), edge("a", "c")];
        assert!(violations(&packages, &edges).is_empty());
    }

    #[test]
    fn a_library_depending_on_a_higher_domain_is_reported() {
        let packages = [package("a", "ui", false), package("s", "shell", false)];
        assert_eq!(
            violations(&packages, &[edge("a", "s")]),
            ["crates/ui/a/Cargo.toml:1: domain-order: a (ui) -> s (shell)"]
        );
    }

    #[test]
    fn a_program_may_depend_on_any_domain_and_is_not_a_dependency_to_avoid() {
        let packages = [
            package("a", "ui", true),
            package("s", "shell", false),
            package("l", "base", false),
            package("p", "shell", true),
        ];
        assert!(violations(&packages, &[edge("a", "s"), edge("l", "p")]).is_empty());
    }

    #[test]
    fn a_domain_outside_the_order_is_reported() {
        let packages = [package("a", "elsewhere", false)];
        assert_eq!(
            violations(&packages, &[]),
            [
                "crates/elsewhere/a/Cargo.toml:1: domain-order: a is in domain `elsewhere`, which the order does not name"
            ]
        );
    }

    #[test]
    fn an_edge_to_a_crate_outside_the_workspace_is_ignored() {
        let packages = [package("a", "ui", false)];
        assert!(violations(&packages, &[edge("a", "serde")]).is_empty());
    }
}
