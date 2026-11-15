//! Pure logic: which packages does a set of changed files affect?

use std::collections::BTreeSet;
use std::path::Path;

use crate::workspace::Workspace;

/// What a change touches.
#[derive(Debug, PartialEq, Eq)]
pub enum Scope {
    /// Something that can change every crate (root manifest, lock, toolchain, cargo config).
    All,
    /// These packages: the owners of the changed files and everything depending on them.
    Packages(BTreeSet<String>),
}

/// Whether `file` (relative to the root) can change how every crate builds.
fn is_global(file: &Path) -> bool {
    matches!(
        file.to_str(),
        Some("Cargo.toml" | "Cargo.lock" | "rust-toolchain" | "rust-toolchain.toml")
    ) || file.starts_with(".cargo")
}

/// The package whose manifest directory is the longest prefix of `file`.
pub fn owner<'a>(ws: &'a Workspace, file: &Path) -> Option<&'a str> {
    ws.dirs
        .iter()
        .filter(|(_, dir)| file.starts_with(dir))
        .max_by_key(|(_, dir)| dir.components().count())
        .map(|(name, _)| name.as_str())
}

/// Computes the [`Scope`] of `changed` (paths relative to the workspace root).
/// Files owned by no package, such as docs, are ignored.
pub fn scope(ws: &Workspace, changed: &[String]) -> Scope {
    let mut seeds = BTreeSet::new();
    for file in changed {
        let path = Path::new(file);
        if is_global(path) {
            return Scope::All;
        }
        if let Some(name) = owner(ws, path) {
            seeds.insert(name.to_owned());
        }
    }
    Scope::Packages(with_reverse_deps(ws, seeds))
}

/// Adds every transitive reverse dependency of `seeds`.
pub fn with_reverse_deps(ws: &Workspace, seeds: BTreeSet<String>) -> BTreeSet<String> {
    let mut seen = seeds;
    let mut queue: Vec<String> = seen.iter().cloned().collect();
    while let Some(name) = queue.pop() {
        for dependent in ws.rdeps.get(&name).into_iter().flatten() {
            if seen.insert(dependent.clone()) {
                queue.push(dependent.clone());
            }
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// core <- mid <- app, plus a nested crate `crates/core/inner` and an unrelated `solo`.
    fn ws() -> Workspace {
        let dirs = [
            ("core", "crates/core"),
            ("inner", "crates/core/inner"),
            ("mid", "crates/mid"),
            ("app", "crates/app"),
            ("solo", "crates/solo"),
            ("xtask", "xtask"),
        ]
        .iter()
        .map(|(n, d)| ((*n).to_owned(), PathBuf::from(d)))
        .collect();
        let edges = [("mid", "core"), ("app", "mid"), ("core", "solo")]
            .iter()
            .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
            .collect::<Vec<_>>();
        Workspace::from_parts(PathBuf::from("/r"), dirs, &edges)
    }

    fn set(names: &[&str]) -> Scope {
        Scope::Packages(names.iter().map(|n| (*n).to_owned()).collect())
    }

    #[test]
    fn longest_prefix_wins() {
        let ws = ws();
        assert_eq!(
            owner(&ws, Path::new("crates/core/inner/src/lib.rs")),
            Some("inner")
        );
        assert_eq!(
            owner(&ws, Path::new("crates/core/src/lib.rs")),
            Some("core")
        );
        assert_eq!(owner(&ws, Path::new("crates/corex/src/lib.rs")), None);
    }

    #[test]
    fn reverse_deps_are_transitive() {
        let got = scope(&ws(), &["crates/solo/src/lib.rs".into()]);
        assert_eq!(got, set(&["solo", "core", "mid", "app"]));
    }

    #[test]
    fn leaf_change_affects_only_itself() {
        assert_eq!(
            scope(&ws(), &["crates/app/src/main.rs".into()]),
            set(&["app"])
        );
        assert_eq!(scope(&ws(), &["xtask/src/main.rs".into()]), set(&["xtask"]));
    }

    #[test]
    fn unowned_files_are_ignored() {
        assert_eq!(
            scope(&ws(), &["README.md".into(), "packaging/x".into()]),
            set(&[])
        );
    }

    #[test]
    fn global_files_mean_all() {
        for f in [
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            ".cargo/config.toml",
        ] {
            assert_eq!(
                scope(&ws(), &["crates/app/a.rs".into(), f.into()]),
                Scope::All
            );
        }
        assert_ne!(scope(&ws(), &["crates/app/Cargo.toml".into()]), Scope::All);
    }
}
