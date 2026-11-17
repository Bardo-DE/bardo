//! `cargo xtask gate`: the workspace-deps, renderer-edges, domain-order and product-words checks, then fmt, clippy and tests on
//! the affected set.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::Result;

use super::{finish, git_lines, run_logged_within};
use crate::affected::{Scope, scope};
use crate::workspace::Workspace;

/// Runs the gate. `base` defaults to `origin/master`, else `HEAD~1`.
pub fn run(base: Option<&str>, all: bool) -> Result<bool> {
    // Reads the manifests only, so it runs on every gate whatever the affected set.
    if !super::workspace_deps::run()? {
        return Ok(finish("gate", false));
    }
    if !super::renderer_edges::run()? {
        return Ok(finish("gate", false));
    }
    if !super::domain_order::run()? {
        return Ok(finish("gate", false));
    }
    if !super::product_words::run()? {
        return Ok(finish("gate", false));
    }
    if !super::shear::run()? {
        return Ok(finish("gate", false));
    }
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
    let tests_root = TestsRoot::new(&ws.root)?;
    for step in steps(&selection) {
        let mut cmd = Command::new("cargo");
        cmd.args(&step.args);
        for var in SESSION_VARS {
            cmd.env_remove(var);
        }
        // A gate builds each sha once: incremental state is only disk (about half the target
        // dir on a whole-workspace run) and is never reused.
        cmd.env("CARGO_INCREMENTAL", "0");
        // No debug info in gate builds: smaller test binaries and faster links. A failing test
        // still names its file and line (panic locations are compiled in), and the same setting
        // on every step keeps one build of each crate.
        cmd.env("CARGO_PROFILE_DEV_DEBUG", "false");
        if step.runs_tests {
            tests_root.apply(&mut cmd, step.args.iter().any(|a| a == "--doc"))?;
        }
        if !run_logged_within(&ws.root, &mut cmd, step.deadline)? {
            return Ok(finish("gate", false));
        }
    }
    Ok(finish("gate", true))
}

/// The folder every test process of this gate gets its throwaway home under (see
/// `xtask/test-env.sh`), removed when the gate ends.
struct TestsRoot {
    dir: PathBuf,
    runner: PathBuf,
}

impl TestsRoot {
    /// Under `/tmp` with a short name, not under the build folder: tests bind Unix sockets (private
    /// buses, compositors) inside their throwaway folders, and a socket path has at most 107 bytes.
    /// A root under `/mnt/data/cargo/<lane>/` left too little room and ~270 tests could not start a
    /// bus ("Socket name too long").
    fn new(root: &Path) -> Result<Self> {
        let dir = PathBuf::from(format!("/tmp/bardo-gate-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            dir,
            runner: root.join("xtask/test-env.sh"),
        })
    }

    /// Points the test processes of `cmd` at `test-env.sh` (as cargo's target runner, which nextest
    /// honours) and that runner at this folder. Doc tests ignore the runner, so for them (`doc`) the
    /// throwaway home is set on the whole step instead, with cargo's and rustup's own homes pinned
    /// first so cargo still finds its toolchain and registry. `XDG_RUNTIME_DIR` stays there: the build
    /// shim keeps its lock in it.
    fn apply(&self, cmd: &mut Command, doc: bool) -> Result<()> {
        cmd.env("BARDO_TEST_ROOT", &self.dir);
        cmd.env(runner_var()?, &self.runner);
        if doc {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
            let pinned = |var: &str, under_home: &str| {
                std::env::var_os(var).map_or_else(|| home.join(under_home), PathBuf::from)
            };
            cmd.env("CARGO_HOME", pinned("CARGO_HOME", ".cargo"));
            cmd.env("RUSTUP_HOME", pinned("RUSTUP_HOME", ".rustup"));
            let doc_home = self.dir.join("doc");
            for (var, sub) in [
                ("HOME", "home"),
                ("XDG_CONFIG_HOME", "config"),
                ("XDG_DATA_HOME", "data"),
                ("XDG_STATE_HOME", "state"),
                ("XDG_CACHE_HOME", "cache"),
            ] {
                let dir = doc_home.join(sub);
                std::fs::create_dir_all(&dir)?;
                cmd.env(var, dir);
            }
            cmd.env(
                "DBUS_SYSTEM_BUS_ADDRESS",
                "unix:path=/nonexistent/bardo-tests",
            );
        }
        Ok(())
    }
}

impl Drop for TestsRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// `CARGO_TARGET_<HOST TRIPLE>_RUNNER`, the variable cargo and nextest read the runner from.
fn runner_var() -> Result<String> {
    let out = Command::new("rustc").arg("-vV").output()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let host = text
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .ok_or_else(|| anyhow::anyhow!("rustc -vV names no host triple"))?;
    Ok(format!(
        "CARGO_TARGET_{}_RUNNER",
        host.to_uppercase().replace(['-', '.'], "_")
    ))
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

/// One gate step: cargo's arguments, how long it may run, and whether it runs tests (and so gets
/// the throwaway-home runner).
#[derive(Debug, PartialEq, Eq)]
struct Step {
    args: Vec<String>,
    deadline: Duration,
    runs_tests: bool,
}

const MINUTE: u64 = 60;

/// The steps, in order. Tests run under nextest with the `gate` profile of `.config/nextest.toml`
/// (a per-test limit, leak detection, no retries); the test binaries are built first in their own
/// step so the test run's deadline measures the tests, not the build. Doc tests, which nextest does
/// not run, follow.
fn steps(selection: &Selection) -> Vec<Step> {
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
    let with_pkgs = |head: &[&str]| -> Vec<String> {
        head.iter()
            .map(|s| (*s).to_owned())
            .chain(pkg_args.iter().cloned())
            .collect()
    };
    let step = |args: Vec<String>, minutes: u64, runs_tests: bool| Step {
        args,
        deadline: Duration::from_secs(minutes * MINUTE),
        runs_tests,
    };
    let mut fmt = vec!["fmt".to_owned(), "--check".to_owned()];
    fmt.extend(fmt_pkgs);
    // Any warning fails the gate: the workspace lint set is the bar, and a warning nobody has to
    // fix only grows into the next backlog.
    let mut clippy = with_pkgs(&["clippy", "--all-targets"]);
    clippy.extend(["--", "-D", "warnings"].map(str::to_owned));
    let mut steps = vec![
        step(fmt, 10, false),
        step(clippy, 150, false),
        step(with_pkgs(&["test", "--no-run"]), 150, false),
        step(
            with_pkgs(&["nextest", "run", "--profile", "gate", "--no-fail-fast"]),
            60,
            true,
        ),
        step(with_pkgs(&["test", "--doc", "--no-fail-fast"]), 30, true),
    ];
    for (pkg, features) in FEATURE_TESTS {
        let wanted = match selection {
            Selection::All => true,
            Selection::Some(pkgs) => pkgs.contains(pkg),
        };
        if wanted {
            let args = [
                "nextest",
                "run",
                "--profile",
                "gate",
                "--no-fail-fast",
                "-p",
                pkg,
                "--features",
                features,
            ];
            steps.push(step(args.map(str::to_owned).to_vec(), 60, true));
        }
    }
    steps
}

/// Tests that only build with a feature a plain workspace run must not unify on (it would
/// change other crates' binaries). Moves to layers.toml's per-crate `features` in phase 2.
const FEATURE_TESTS: [(&str, &str); 1] = [("account-secrets", "test-keys")];

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
        assert_eq!(s[0].args, ["fmt", "--check", "--all"]);
        assert_eq!(
            s[1].args,
            [
                "clippy",
                "--all-targets",
                "--workspace",
                "--",
                "-D",
                "warnings"
            ]
        );
        assert_eq!(s[2].args, ["test", "--no-run", "--workspace"]);
        assert_eq!(
            s[3].args,
            [
                "nextest",
                "run",
                "--profile",
                "gate",
                "--no-fail-fast",
                "--workspace"
            ]
        );
        assert_eq!(
            s[4].args,
            ["test", "--doc", "--no-fail-fast", "--workspace"]
        );
    }

    #[test]
    fn some_uses_package_flags() {
        let pkgs = ["a", "b"].iter().map(|s| (*s).to_owned()).collect();
        let s = steps(&Selection::Some(pkgs));
        assert_eq!(s[0].args, ["fmt", "--check", "-p", "a", "-p", "b"]);
        assert_eq!(
            s[1].args,
            [
                "clippy",
                "--all-targets",
                "-p",
                "a",
                "-p",
                "b",
                "--",
                "-D",
                "warnings"
            ]
        );
    }

    #[test]
    fn every_step_has_a_deadline_and_only_test_steps_get_the_runner() {
        for step in steps(&Selection::All) {
            assert!(step.deadline >= Duration::from_secs(MINUTE), "{step:?}");
            let tests = step.args.first().map(String::as_str) == Some("nextest")
                || step.args.iter().any(|a| a == "--doc");
            assert_eq!(step.runs_tests, tests, "{step:?}");
        }
    }
}
