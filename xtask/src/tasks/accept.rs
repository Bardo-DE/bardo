//! `cargo xtask accept [<suite>] [<filter>]` and `cargo xtask live <suite> [<filter>] --yes`: the
//! suites of `suites.toml`.
//!
//! An accept suite is the `accept::` tests of its packages, run inside `packaging/harness/jail.sh`
//! (private bus, scratch home, no devices, no network). A live suite is the `live::` tests, run on
//! the real machine, only with `--yes`, never by a gate. The programs a suite names as prebuilds are
//! built first, each into its own target folder per feature set (so its features never unify with
//! the workspace build), and handed to the tests as `BARDO_ACCEPT_BIN_<BIN>`.
//!
//! The tests are built outside the jail into a nextest archive, and only the archive runs inside:
//! the jail has a read-only filesystem and a scratch home, where cargo could not work.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use super::{finish, run_logged};
use crate::workspace::Workspace;

/// The exit code `jail.sh` uses for "the jail cannot be made on this machine".
const JAIL_UNAVAILABLE: i32 = 3;

/// Which kind of tests a suite holds, and so how it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Jailed and hermetic: the `accept::` tests.
    Accept,
    /// Touches something real: the `live::` tests.
    Live,
}

impl Kind {
    /// The command word, the nextest profile and the test module prefix.
    const fn word(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Live => "live",
        }
    }
}

/// A program built before the run.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prebuild {
    /// The package that has the binary.
    pub package: String,
    /// The binary's name.
    pub bin: String,
    /// The features to build it with.
    #[serde(default)]
    pub features: Vec<String>,
}

/// One suite of `suites.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Suite {
    /// Accept or live.
    pub kind: Kind,
    /// The packages whose tests make the suite.
    pub packages: Vec<String>,
    /// What it proves (accept) or touches (live), in a plain sentence.
    pub why: String,
    /// Programs to build first.
    #[serde(default)]
    pub prebuild: Vec<Prebuild>,
}

/// The parsed `suites.toml`, by suite name.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    suite: BTreeMap<String, Suite>,
}

/// Parses the text of `suites.toml`.
fn parse(text: &str) -> Result<BTreeMap<String, Suite>> {
    let manifest: Manifest = toml::from_str(text).context("suites.toml is not valid")?;
    for (name, suite) in &manifest.suite {
        if suite.packages.is_empty() {
            bail!("suites.toml: suite `{name}` has no packages");
        }
    }
    Ok(manifest.suite)
}

/// `BARDO_ACCEPT_BIN_<BIN IN UPPER_SNAKE>`: `bardo-accountd` is `BARDO_ACCEPT_BIN_BARDO_ACCOUNTD`.
fn bin_var(bin: &str) -> String {
    let snake: String = bin
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    format!("BARDO_ACCEPT_BIN_{snake}")
}

/// The target folder of one prebuild: `<target>/accept/<package>-<features joined by +>`.
fn prebuild_dir(target: &Path, p: &Prebuild) -> PathBuf {
    let features = if p.features.is_empty() {
        "default".to_owned()
    } else {
        p.features.join("+")
    };
    target
        .join("accept")
        .join(format!("{}-{features}", p.package))
}

/// Where the build leaves the binary of `p`.
fn prebuilt_path(target: &Path, p: &Prebuild) -> PathBuf {
    prebuild_dir(target, p).join("debug").join(&p.bin)
}

/// The cargo arguments that build `p` into its own folder.
fn prebuild_args(target: &Path, p: &Prebuild) -> Vec<String> {
    let mut args: Vec<String> = [
        "build",
        "--locked",
        "-p",
        &p.package,
        "--bin",
        &p.bin,
        "--target-dir",
    ]
    .map(str::to_owned)
    .to_vec();
    args.push(prebuild_dir(target, p).display().to_string());
    if !p.features.is_empty() {
        args.push("--features".to_owned());
        args.push(p.features.join(","));
    }
    args
}

/// The nextest filter expression: the kind's tests, and the substring `filter` when given.
fn filter_expr(kind: Kind, filter: Option<&str>) -> Result<String> {
    let base = format!("test(/^{}::/)", kind.word());
    match filter {
        None => Ok(base),
        Some(f) if f.is_empty() || f.contains(['(', ')', '/', ' ']) => {
            bail!(
                "the filter `{f}` must be a plain part of a test name (no spaces, parentheses or slashes)"
            )
        }
        Some(f) => Ok(format!("{base} and test(~{f})")),
    }
}

/// The `-p` arguments for `packages`.
fn package_args(packages: &[String]) -> Vec<String> {
    packages
        .iter()
        .flat_map(|p| ["-p".to_owned(), p.clone()])
        .collect()
}

/// The cargo arguments that build the suite's tests into the nextest archive `archive`.
fn archive_args(suite: &Suite, archive: &Path) -> Vec<String> {
    let mut args: Vec<String> = ["nextest", "archive", "--locked"]
        .map(str::to_owned)
        .to_vec();
    args.extend(package_args(&suite.packages));
    args.push("--archive-file".to_owned());
    args.push(archive.display().to_string());
    args
}

/// The nextest arguments (after `nextest`) of the run itself.
fn nextest_args(kind: Kind, expr: &str) -> Vec<String> {
    [
        "run",
        "--profile",
        kind.word(),
        "--no-fail-fast",
        "-E",
        expr,
    ]
    .map(str::to_owned)
    .to_vec()
}

/// The whole jail command line: `jail.sh --bind root --bind target --env K=V ... -- cargo-nextest
/// nextest run --archive-file ...`. The workspace and the target folder are bound writable; the
/// prebuilt programs are reached through the read-only rest of the filesystem.
fn jail_argv(
    jail: &Path,
    root: &Path,
    target: &Path,
    archive: &Path,
    env: &[(String, String)],
    expr: &str,
) -> Vec<String> {
    let mut argv = vec![
        jail.display().to_string(),
        "--bind".to_owned(),
        root.display().to_string(),
        "--bind".to_owned(),
        target.display().to_string(),
    ];
    for (k, v) in env {
        argv.push("--env".to_owned());
        argv.push(format!("{k}={v}"));
    }
    argv.push("--".to_owned());
    argv.extend(["cargo-nextest", "nextest"].map(str::to_owned));
    argv.extend(nextest_args(Kind::Accept, expr));
    argv.extend(["--archive-file".to_owned(), archive.display().to_string()]);
    argv.extend(["--workspace-remap".to_owned(), root.display().to_string()]);
    argv
}

/// The cargo target directory: `CARGO_TARGET_DIR`, else `<root>/target`.
fn target_dir(root: &Path) -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

/// Reads `suites.toml` at the workspace root.
fn load(root: &Path) -> Result<BTreeMap<String, Suite>> {
    let file = root.join("suites.toml");
    let text = std::fs::read_to_string(&file)
        .with_context(|| format!("could not read {}", file.display()))?;
    parse(&text)
}

/// `cargo xtask accept --list` / `live --list`: the suites of `kind`, one per line with their why.
pub fn list(kind: Kind) -> Result<bool> {
    let ws = Workspace::load()?;
    for (name, suite) in load(&ws.root)? {
        if suite.kind == kind {
            println!("{name}: {} ({})", suite.why, suite.packages.join(", "));
        }
    }
    Ok(true)
}

/// Runs the accept suite `name`, or every accept suite when `name` is `None`.
pub fn run_accept(name: Option<&str>, filter: Option<&str>) -> Result<bool> {
    let ws = Workspace::load()?;
    let suites = load(&ws.root)?;
    let chosen = choose(&suites, Kind::Accept, name)?;
    let label = name.map_or_else(|| "accept".to_owned(), |n| format!("accept {n}"));
    for (suite_name, suite) in chosen {
        if !run_suite(&ws.root, suite_name, suite, filter)? {
            return Ok(finish(&label, false));
        }
    }
    Ok(finish(&label, true))
}

/// Runs the live suite `name`; refuses without `yes`.
pub fn run_live(name: &str, filter: Option<&str>, yes: bool) -> Result<bool> {
    let ws = Workspace::load()?;
    let suites = load(&ws.root)?;
    let chosen = choose(&suites, Kind::Live, Some(name))?;
    let label = format!("live {name}");
    for (suite_name, suite) in chosen {
        println!(
            "xtask: live {suite_name} touches the real machine: {}",
            suite.why
        );
        if !yes {
            eprintln!("xtask: not running it without --yes");
            return Ok(finish(&label, false));
        }
        if !run_suite(&ws.root, suite_name, suite, filter)? {
            return Ok(finish(&label, false));
        }
    }
    Ok(finish(&label, true))
}

/// The suites to run: `name` (which must be of `kind`), or every suite of `kind`.
fn choose<'a>(
    suites: &'a BTreeMap<String, Suite>,
    kind: Kind,
    name: Option<&str>,
) -> Result<Vec<(&'a String, &'a Suite)>> {
    match name {
        Some(n) => match suites.get_key_value(n) {
            Some((k, s)) if s.kind == kind => Ok(vec![(k, s)]),
            Some(_) => bail!(
                "`{n}` is not a {} suite (see `cargo xtask {} --list`)",
                kind.word(),
                kind.word()
            ),
            None => bail!(
                "no suite `{n}` in suites.toml (see `cargo xtask {} --list`)",
                kind.word()
            ),
        },
        None => Ok(suites.iter().filter(|(_, s)| s.kind == kind).collect()),
    }
}

/// Builds the prebuilds, then runs one suite. `Ok(false)` is a failure already reported.
fn run_suite(root: &Path, name: &str, suite: &Suite, filter: Option<&str>) -> Result<bool> {
    let expr = filter_expr(suite.kind, filter)?;
    let target = target_dir(root);
    println!("xtask: {} {name}: {}", suite.kind.word(), suite.why);
    let mut env = Vec::new();
    for p in &suite.prebuild {
        let mut cmd = Command::new("cargo");
        cmd.args(prebuild_args(&target, p));
        if !run_logged(root, &mut cmd)? {
            return Ok(false);
        }
        env.push((
            bin_var(&p.bin),
            prebuilt_path(&target, p).display().to_string(),
        ));
    }
    match suite.kind {
        Kind::Live => {
            let mut cmd = Command::new("cargo");
            cmd.arg("nextest")
                .args(nextest_args(Kind::Live, &expr))
                .args(package_args(&suite.packages))
                .envs(env);
            run_logged(root, &mut cmd)
        }
        Kind::Accept => run_jailed(root, &target, name, suite, &env, &expr),
    }
}

/// Archives the tests outside the jail, then runs the archive inside it.
fn run_jailed(
    root: &Path,
    target: &Path,
    name: &str,
    suite: &Suite,
    env: &[(String, String)],
    expr: &str,
) -> Result<bool> {
    let dir = target.join("accept");
    std::fs::create_dir_all(&dir)?;
    let archive = dir.join(format!("{name}.tar.zst"));
    let mut build = Command::new("cargo");
    build.args(archive_args(suite, &archive));
    if !run_logged(root, &mut build)? {
        return Ok(false);
    }
    let mut env = env.to_vec();
    for var in ["CARGO_HOME", "RUSTUP_HOME"] {
        if let Some(v) = std::env::var_os(var) {
            env.push((var.to_owned(), v.to_string_lossy().into_owned()));
        }
    }
    let jail = root.join("packaging/harness/jail.sh");
    let argv = jail_argv(&jail, root, target, &archive, &env, expr);
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    println!("xtask: $ {cmd:?}");
    let status = cmd
        .current_dir(root)
        .status()
        .with_context(|| format!("could not start {}", jail.display()))?;
    if status.code() == Some(JAIL_UNAVAILABLE) {
        eprintln!(
            "xtask: accept {name}: the jail is unavailable on this machine (the reason and the fix are printed above)"
        );
        return Ok(false);
    }
    Ok(status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[suite.agent]
kind = "accept"
packages = ["action-accept"]
why = "end to end"
[[suite.agent.prebuild]]
package = "account-daemon"
bin = "bardo-accountd"
features = ["test-proc-root", "test-keys"]

[suite.gpu]
kind = "live"
packages = ["ui-harness"]
why = "renders"
"#;

    fn accountd() -> Prebuild {
        Prebuild {
            package: "account-daemon".into(),
            bin: "bardo-accountd".into(),
            features: vec!["test-proc-root".into(), "test-keys".into()],
        }
    }

    #[test]
    fn parses_suites_and_prebuilds() {
        let suites = parse(SAMPLE).unwrap();
        assert_eq!(suites["agent"].kind, Kind::Accept);
        assert_eq!(suites["agent"].prebuild, [accountd()]);
        assert_eq!(suites["gpu"].kind, Kind::Live);
        assert!(suites["gpu"].prebuild.is_empty());
    }

    #[test]
    fn rejects_unknown_keys_and_empty_package_lists() {
        parse("[suite.a]\nkind=\"accept\"\npackages=[\"x\"]\nwhy=\"w\"\nfeature=1").unwrap_err();
        parse("[suite.a]\nkind=\"accept\"\npackages=[]\nwhy=\"w\"").unwrap_err();
        parse("[suite.a]\nkind=\"other\"\npackages=[\"x\"]\nwhy=\"w\"").unwrap_err();
    }

    #[test]
    fn the_checked_in_suites_toml_parses() {
        let text = include_str!("../../../suites.toml");
        let suites = parse(text).unwrap();
        assert!(suites.values().all(|s| !s.why.is_empty()));
    }

    #[test]
    fn bin_names_map_to_env_vars() {
        assert_eq!(bin_var("bardo-accountd"), "BARDO_ACCEPT_BIN_BARDO_ACCOUNTD");
        assert_eq!(bin_var("a.b-c"), "BARDO_ACCEPT_BIN_A_B_C");
    }

    #[test]
    fn prebuilds_get_their_own_target_folder() {
        let target = Path::new("/t");
        assert_eq!(
            prebuild_dir(target, &accountd()),
            Path::new("/t/accept/account-daemon-test-proc-root+test-keys")
        );
        assert_eq!(
            prebuilt_path(target, &accountd()),
            Path::new("/t/accept/account-daemon-test-proc-root+test-keys/debug/bardo-accountd")
        );
        let plain = Prebuild {
            features: vec![],
            ..accountd()
        };
        assert_eq!(
            prebuild_dir(target, &plain),
            Path::new("/t/accept/account-daemon-default")
        );
    }

    #[test]
    fn prebuild_command_line() {
        assert_eq!(
            prebuild_args(Path::new("/t"), &accountd()),
            [
                "build",
                "--locked",
                "-p",
                "account-daemon",
                "--bin",
                "bardo-accountd",
                "--target-dir",
                "/t/accept/account-daemon-test-proc-root+test-keys",
                "--features",
                "test-proc-root,test-keys",
            ]
        );
        let plain = Prebuild {
            features: vec![],
            ..accountd()
        };
        assert!(!prebuild_args(Path::new("/t"), &plain).contains(&"--features".to_owned()));
    }

    #[test]
    fn filter_expression() {
        assert_eq!(
            filter_expr(Kind::Accept, None).unwrap(),
            "test(/^accept::/)"
        );
        assert_eq!(
            filter_expr(Kind::Live, Some("clipboard")).unwrap(),
            "test(/^live::/) and test(~clipboard)"
        );
        filter_expr(Kind::Accept, Some("a) | test(b")).unwrap_err();
        filter_expr(Kind::Accept, Some("")).unwrap_err();
    }

    #[test]
    fn archive_command_line() {
        let suite = parse(SAMPLE).unwrap().remove("agent").unwrap();
        assert_eq!(
            archive_args(&suite, Path::new("/t/accept/agent.tar.zst")),
            [
                "nextest",
                "archive",
                "--locked",
                "-p",
                "action-accept",
                "--archive-file",
                "/t/accept/agent.tar.zst"
            ]
        );
    }

    #[test]
    fn jail_command_line() {
        let env = vec![(
            "BARDO_ACCEPT_BIN_BARDO_ACCOUNTD".to_owned(),
            "/t/x".to_owned(),
        )];
        let argv = jail_argv(
            Path::new("/r/packaging/harness/jail.sh"),
            Path::new("/r"),
            Path::new("/t"),
            Path::new("/t/accept/agent.tar.zst"),
            &env,
            "test(/^accept::/)",
        );
        assert_eq!(
            argv,
            [
                "/r/packaging/harness/jail.sh",
                "--bind",
                "/r",
                "--bind",
                "/t",
                "--env",
                "BARDO_ACCEPT_BIN_BARDO_ACCOUNTD=/t/x",
                "--",
                "cargo-nextest",
                "nextest",
                "run",
                "--profile",
                "accept",
                "--no-fail-fast",
                "-E",
                "test(/^accept::/)",
                "--archive-file",
                "/t/accept/agent.tar.zst",
                "--workspace-remap",
                "/r",
            ]
        );
    }

    #[test]
    fn choosing_checks_the_kind() {
        let suites = parse(SAMPLE).unwrap();
        assert_eq!(choose(&suites, Kind::Accept, None).unwrap().len(), 1);
        choose(&suites, Kind::Live, Some("agent")).unwrap_err();
        choose(&suites, Kind::Accept, Some("nope")).unwrap_err();
        assert_eq!(choose(&suites, Kind::Live, Some("gpu")).unwrap().len(), 1);
    }
}
