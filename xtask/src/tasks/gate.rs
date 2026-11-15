//! `cargo xtask gate`: fmt, clippy and tests on the affected set.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use anyhow::Result;

use super::{finish, git_lines, run_logged};
use crate::affected::{Scope, scope};
use crate::workspace::Workspace;

/// Runs the gate. `base` defaults to `origin/master`, else `HEAD~1`.
pub fn run(base: Option<&str>, all: bool) -> Result<bool> {
    let ws = Workspace::load()?;
    let scope = if all {
        Scope::All
    } else {
        let changed = changed_files(&ws.root, base)?;
        scope(&ws, &changed)
    };
    let selection = match scope {
        Scope::All => {
            println!("xtask: gate: whole workspace");
            Selection::All
        }
        Scope::Packages(pkgs) if pkgs.is_empty() => {
            println!("xtask: gate: no affected packages, nothing to do");
            return Ok(finish("gate", true));
        }
        Scope::Packages(pkgs) => {
            println!("xtask: gate: {} affected package(s)", pkgs.len());
            for name in &pkgs {
                println!("xtask:   {name}");
            }
            Selection::Some(pkgs)
        }
    };
    for step in steps(&selection) {
        let mut cmd = Command::new("cargo");
        cmd.args(&step);
        for var in SESSION_VARS {
            cmd.env_remove(var);
        }
        if !run_logged(&ws.root, &mut cmd)? {
            return Ok(finish("gate", false));
        }
    }
    Ok(finish("gate", true))
}

/// The person's display and session bus, removed before any step runs so no test can reach the
/// running desktop (tests start their own private bus and headless compositor). `XDG_RUNTIME_DIR`
/// stays: the build shim keeps its lock there.
const SESSION_VARS: [&str; 4] = [
    "DISPLAY",
    "WAYLAND_DISPLAY",
    "WAYLAND_SOCKET",
    "DBUS_SESSION_BUS_ADDRESS",
];

/// Which packages the steps cover.
enum Selection {
    All,
    Some(BTreeSet<String>),
}

/// The cargo argument lists of each step, in order.
fn steps(selection: &Selection) -> Vec<Vec<String>> {
    let pkg_args: Vec<String> = match selection {
        Selection::All => vec!["--workspace".into()],
        Selection::Some(pkgs) => pkgs
            .iter()
            .flat_map(|p| ["-p".to_owned(), p.clone()])
            .collect(),
    };
    let fmt_pkgs: Vec<String> = match selection {
        Selection::All => vec!["--all".into()],
        Selection::Some(_) => pkg_args.clone(),
    };
    let build = |head: &[&str]| -> Vec<String> {
        head.iter()
            .map(|s| (*s).to_owned())
            .chain(pkg_args.iter().cloned())
            .collect()
    };
    let mut fmt = vec!["fmt".to_owned(), "--check".to_owned()];
    fmt.extend(fmt_pkgs);
    let mut steps = vec![
        fmt,
        build(&["clippy", "--all-targets"]),
        build(&["test", "--no-fail-fast"]),
    ];
    for (pkg, features) in FEATURE_TESTS {
        let wanted = match selection {
            Selection::All => true,
            Selection::Some(pkgs) => pkgs.contains(pkg),
        };
        if wanted {
            steps.push(
                ["test", "--no-fail-fast", "-p", pkg, "--features", features]
                    .map(str::to_owned)
                    .to_vec(),
            );
        }
    }
    steps
}

/// Tests that only build with a feature a plain workspace run must not unify on (it would
/// change other crates' binaries). Moves to layers.toml's per-crate `features` in phase 2.
const FEATURE_TESTS: [(&str, &str); 1] = [("porter-secrets", "test-keys")];

/// Files changed against `base`, committed, staged, unstaged or untracked, relative to `root`.
fn changed_files(root: &Path, base: Option<&str>) -> Result<Vec<String>> {
    let base = match base {
        Some(b) => b.to_owned(),
        None if git_lines(root, &["rev-parse", "--verify", "--quiet", "origin/master"])?
            .is_some() =>
        {
            "origin/master".to_owned()
        }
        None => "HEAD~1".to_owned(),
    };
    let range = format!("{base}...HEAD");
    let mut files = BTreeSet::new();
    for args in [
        vec!["diff", "--name-only", range.as_str()],
        vec!["diff", "--name-only", "HEAD"],
        vec!["ls-files", "--others", "--exclude-standard"],
    ] {
        let lines = git_lines(root, &args)?.ok_or_else(|| {
            anyhow::anyhow!(
                "`git {}` failed (is `{base}` a valid revision?)",
                args.join(" ")
            )
        })?;
        files.extend(lines);
    }
    Ok(files.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_uses_workspace_flags() {
        let s = steps(&Selection::All);
        assert_eq!(s[0], ["fmt", "--check", "--all"]);
        assert_eq!(s[1], ["clippy", "--all-targets", "--workspace"]);
        assert_eq!(s[2], ["test", "--no-fail-fast", "--workspace"]);
    }

    #[test]
    fn some_uses_package_flags() {
        let pkgs = ["a", "b"].iter().map(|s| (*s).to_owned()).collect();
        let s = steps(&Selection::Some(pkgs));
        assert_eq!(s[0], ["fmt", "--check", "-p", "a", "-p", "b"]);
        assert_eq!(s[1], ["clippy", "--all-targets", "-p", "a", "-p", "b"]);
    }
}
