//! The workspace as plain data, loaded once from `cargo metadata`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;

/// Every workspace member's location and who depends on it.
#[derive(Debug, Clone, Default)]
pub struct Workspace {
    /// Absolute workspace root.
    pub root: PathBuf,
    /// Package name to manifest directory, relative to [`Workspace::root`].
    pub dirs: BTreeMap<String, PathBuf>,
    /// Package name to the workspace members that depend on it directly
    /// (normal, build and dev dependencies alike).
    pub rdeps: BTreeMap<String, BTreeSet<String>>,
}

impl Workspace {
    /// Loads the workspace containing the current directory.
    ///
    /// # Errors
    ///
    /// Fails if `cargo metadata` fails or a member lies outside the root.
    pub fn load() -> Result<Self> {
        let meta = MetadataCommand::new()
            .no_deps()
            .exec()
            .context("cargo metadata failed")?;
        let root = meta.workspace_root.clone().into_std_path_buf();
        let mut dirs = BTreeMap::new();
        let mut edges = Vec::new();
        for pkg in &meta.packages {
            let dir = pkg
                .manifest_path
                .parent()
                .context("manifest path has no parent")?
                .as_std_path();
            let rel = dir
                .strip_prefix(&root)
                .with_context(|| format!("{} is outside the workspace", pkg.name))?;
            dirs.insert(pkg.name.to_string(), rel.to_path_buf());
            for dep in &pkg.dependencies {
                edges.push((pkg.name.to_string(), dep.name.clone()));
            }
        }
        Ok(Self::from_parts(root, dirs, &edges))
    }

    /// Builds a workspace from member directories and `(dependent, dependency)`
    /// edges. Edges to names that are not members are dropped.
    pub fn from_parts(
        root: PathBuf,
        dirs: BTreeMap<String, PathBuf>,
        edges: &[(String, String)],
    ) -> Self {
        let mut rdeps: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (dependent, dependency) in edges {
            if dirs.contains_key(dependency) && dependent != dependency {
                rdeps
                    .entry(dependency.clone())
                    .or_default()
                    .insert(dependent.clone());
            }
        }
        Self { root, dirs, rdeps }
    }

    /// The directory of `name` relative to the root, if it is a member.
    #[allow(dead_code)]
    pub fn dir_of(&self, name: &str) -> Option<&Path> {
        self.dirs.get(name).map(PathBuf::as_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirs(names: &[(&str, &str)]) -> BTreeMap<String, PathBuf> {
        names
            .iter()
            .map(|(n, d)| ((*n).to_owned(), PathBuf::from(d)))
            .collect()
    }

    #[test]
    fn reverse_edges_only_between_members() {
        let ws = Workspace::from_parts(
            PathBuf::from("/r"),
            dirs(&[("a", "crates/a"), ("b", "crates/b")]),
            &[
                ("b".into(), "a".into()),
                ("b".into(), "serde".into()),
                ("a".into(), "a".into()),
            ],
        );
        assert_eq!(ws.rdeps.get("a").map(BTreeSet::len), Some(1));
        assert!(!ws.rdeps.contains_key("serde"));
        assert_eq!(ws.dir_of("b"), Some(Path::new("crates/b")));
    }
}
