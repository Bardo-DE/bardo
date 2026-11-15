//! `cargo xtask check legacy`: the old per-repo `check-*.sh` scripts, unchanged.
//!
//! They stay until phase 2 replaces them; some assume the old repo layout and will fail.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use super::finish;
use crate::workspace::Workspace;

/// A legacy script: the old repo directory name and the script path inside it.
#[derive(Debug, PartialEq, Eq)]
struct Script {
    repo: String,
    rel: String,
}

impl Script {
    /// `legacy <repo>/<script>` as printed in the report.
    fn label(&self) -> String {
        format!("legacy {}/{}", self.repo, self.rel)
    }
}

/// Whether a file name is a legacy check script.
fn is_check_script(name: &str) -> bool {
    name.starts_with("check-") && name.ends_with(".sh")
}

/// Finds `crates/*/{dev,scripts}/check-*.sh` under `root`, sorted, optionally for one repo.
fn discover(root: &Path, only: Option<&str>) -> Result<Vec<Script>> {
    let mut found = Vec::new();
    let crates = root.join("crates");
    for repo in fs::read_dir(&crates).with_context(|| format!("reading {}", crates.display()))? {
        let repo = repo?.file_name().to_string_lossy().into_owned();
        if only.is_some_and(|o| o != repo) {
            continue;
        }
        for sub in ["dev", "scripts"] {
            let dir: PathBuf = crates.join(&repo).join(sub);
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries {
                let name = entry?.file_name().to_string_lossy().into_owned();
                if is_check_script(&name) {
                    found.push(Script {
                        repo: repo.clone(),
                        rel: format!("{sub}/{name}"),
                    });
                }
            }
        }
    }
    found.sort_by(|a, b| (&a.repo, &a.rel).cmp(&(&b.repo, &b.rel)));
    Ok(found)
}

/// Runs each script with its old repo directory as cwd and reports ok or FAIL per script.
pub fn run(only: Option<&str>) -> Result<bool> {
    let ws = Workspace::load()?;
    let scripts = discover(&ws.root, only)?;
    if scripts.is_empty() {
        println!("xtask: legacy: no scripts found");
    }
    let mut green = true;
    for script in &scripts {
        let dir = ws.root.join("crates").join(&script.repo);
        let ok = Command::new("bash")
            .arg(&script.rel)
            .current_dir(&dir)
            .status()
            .is_ok_and(|s| s.success());
        println!("{}: {}", script.label(), if ok { "ok" } else { "FAIL" });
        green &= ok;
    }
    Ok(finish("check legacy", green))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_names() {
        assert!(is_check_script("check-boundary.sh"));
        assert!(!is_check_script("gate.sh"));
        assert!(!is_check_script("check-x.py"));
    }

    #[test]
    fn label_format() {
        let s = Script {
            repo: "porter".into(),
            rel: "scripts/check-features.sh".into(),
        };
        assert_eq!(s.label(), "legacy porter/scripts/check-features.sh");
    }
}
