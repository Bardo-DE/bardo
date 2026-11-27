//! `cargo xtask check features [--cross]`: the builds a consumer makes on its own.
//!
//! The workspace gate builds with every feature on at once, so it never sees the build a program
//! makes with one feature and no other (a consumer that took one feature once left two methods
//! unused and the gate passed). This check builds each package by itself, with cargo-hack:
//!
//! - clippy with `-D warnings` over each feature alone (or every combination, where a crate's
//!   features interact), on the targets a consumer compiles: `--lib --bins`;
//! - `cargo hack check --no-default-features --all-targets` on the portable core of each domain,
//!   so a use of a desktop-only crate that is not behind its feature fails to build;
//! - with `--cross`: `cargo check --target <t>` on the crates that must build on macOS and
//!   Windows. A target rustup does not have installed is said and skipped, never a failure.
//!
//! The package tables below are the single place the lists live. Errors are
//! `path:line: features: message` lines.

use std::collections::BTreeSet;
use std::process::{Command, Output, Stdio};

use anyhow::{Context, Result};

use super::{finish, run_logged};
use crate::workspace::Workspace;

/// This file, as error lines name it.
const SELF_PATH: &str = "xtask/src/tasks/features.rs";

/// How a package's features are swept by clippy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sweep {
    /// Each feature alone (and none).
    Each,
    /// Every combination of features (the crate's features interact).
    Powerset,
}

// ---- clippy sweeps -------------------------------------------------------------------------

/// The accounts service's crates, in the feature sets their consumers (mailo, syncd, the Settings
/// app, the shell, the actions service, the memory service) build. The daemon alone, so no other
/// package's features are unified in; the client and its two transports; the crates a consumer
/// takes with `default-features = false`.
const ACCOUNTS_CLIPPY: &[(&str, Sweep)] = &[
    ("account-daemon", Sweep::Each),
    ("account-client", Sweep::Each),
    ("account-client-dbus", Sweep::Each),
    ("account-client-inproc", Sweep::Each),
    ("account-secrets", Sweep::Each),
    ("account-discover", Sweep::Each),
    ("http-core", Sweep::Each),
    ("account-oauth", Sweep::Each),
    ("account-proxy", Sweep::Each),
];

/// The inference service: inferd alone, and the client in the sets its consumers name (`dbus`
/// for inferd's tests, the actions service and the memory service; `dbus,lending` for the
/// Settings app), so every combination.
const INFERENCE_CLIPPY: &[(&str, Sweep)] = &[
    ("inference-daemon", Sweep::Each),
    ("inference-client", Sweep::Powerset),
];

/// The sync service: the daemon (with its off-by-default Photos Picker), the engine and the
/// journal with their test seams, the bus interfaces, the helper and the library it is built from.
const SYNC_CLIPPY: &[(&str, Sweep)] = &[
    ("sync-daemon", Sweep::Each),
    ("sync-engine", Sweep::Each),
    ("sync-journal-sqlite", Sweep::Each),
    ("sync-bus", Sweep::Each),
    ("drive-mount-daemon", Sweep::Each),
    ("drive-fuse", Sweep::Each),
];

// ---- portable cores ------------------------------------------------------------------------

/// A package of a portable core and the features its portable build turns on (besides none).
#[derive(Debug, Clone, Copy)]
struct Portable {
    package: &'static str,
    features: &'static [&'static str],
}

const fn bare(package: &'static str) -> Portable {
    Portable {
        package,
        features: &[],
    }
}

/// The accounts core: no desktop bus. `account-client` is the build an accounts-only consumer uses.
const ACCOUNTS_PORTABLE: &[Portable] = &[
    bare("account-client"),
    bare("account-client-inproc"),
    bare("account-core"),
    bare("account-provider"),
    bare("account-secrets"),
    bare("account-oauth"),
    bare("account-families"),
];

/// The actions core: the vocabulary, the router and its Cedar policy point, the reviewer, the agent
/// loop, the planner over any inference-client Transport, skills, the red team's suite, the
/// fakes, the model-backed reviewer and policy writer, the quarantined reader, the memory seam.
/// The desktop extras (the bus crates, the daemons, the acceptance crate) are not here on purpose.
const ACTIONS_PORTABLE: &[Portable] = &[
    bare("action-vocab"),
    bare("action-manifest"),
    bare("permission-core"),
    bare("companion-model"),
    bare("action-core"),
    bare("action-gate-wire"),
    bare("action-wire"),
    bare("agent-skills"),
    bare("agent-skills-disk"),
    bare("permission-policy"),
    bare("action-review"),
    bare("action-context"),
    bare("action-gate"),
    bare("action-router"),
    bare("action-tools"),
    bare("companion-wire"),
    bare("companion-core"),
    bare("agent-loop"),
    bare("agent-planner"),
    bare("companion-tasks"),
    bare("agent-native"),
    bare("agent-session"),
    bare("action-checkpoint"),
    bare("agent-acp-server"),
    bare("action-client"),
    bare("action-fake"),
    bare("action-eval"),
    bare("action-eval-core"),
    bare("agent-models"),
    bare("agent-reader"),
    bare("companion-memory-core"),
    bare("companion-memory"),
    bare("agent-registry"),
    bare("action-host"),
    bare("speech-wire"),
    bare("speech-loop"),
    bare("speech-audio"),
];

/// The memory core, which an app on macOS, Windows or another desktop hosts in process.
/// `memory-client`'s in-process transport is its portable path, so it takes `in_process` (its
/// default-off `dbus` feature is the desktop's). `memory-watch` keeps the seam and the pure `join`;
/// its `linux` feature (the inotify watcher) is off here.
const MEMORY_PORTABLE: &[Portable] = &[
    bare("memory-core"),
    bare("memory-seal"),
    bare("memory-store"),
    bare("memory-eventlog"),
    bare("memory-files"),
    bare("memory-files-format"),
    bare("memory-recall"),
    bare("memory-settings"),
    bare("memory-tidy"),
    bare("memory-search"),
    bare("memory-service"),
    bare("memory-host"),
    bare("memory-watch"),
    Portable {
        package: "memory-client",
        features: &["in_process"],
    },
    bare("memory-local"),
];

/// The inference crates that are not the bus or the daemon.
const INFERENCE_PORTABLE: &[Portable] = &[
    bare("inference-core"),
    bare("inference-bridge"),
    bare("inference-router"),
    bare("inference-fs"),
    bare("inference-settings"),
    bare("inference-agent-proto"),
    bare("inference-turns"),
    bare("inference-tailnet"),
    bare("inference-probe"),
    bare("inference-client"),
];

/// The viewer's embedders: `anyview-peek` alone as the shell and other embedders take it (a
/// workspace build unifies features, so a peek that needs a feature only the viewer turns on
/// compiles there but not in a launcher), headless and with `media`, the launcher's set; and the
/// platform crate, which must build without `desktop`.
const ANYVIEW_PORTABLE: &[Portable] = &[
    bare("anyview-peek"),
    Portable {
        package: "anyview-peek",
        features: &["media"],
    },
    bare("anyview-platform"),
];

/// The UI kit: every crate with a `desktop` feature (default on) builds without it, so no core
/// module reaches the desktop's D-Bus clients (ui-settings, ui-helpers and ui-blitz keep theirs
/// behind `desktop`). Replaces the workspace-wide `--no-default-features` build, which is too slow.
const UI_PORTABLE: &[Portable] = &[
    bare("ui-settings"),
    bare("ui-helpers"),
    bare("ui-blitz"),
    bare("ui-harness"),
    bare("ui-conformance"),
    bare("ui-icon-tool"),
    bare("ui-gallery"),
];

/// The portable cores, by domain.
const PORTABLE: &[(&str, &[Portable])] = &[
    ("accounts", ACCOUNTS_PORTABLE),
    ("actions", ACTIONS_PORTABLE),
    ("memory", MEMORY_PORTABLE),
    ("inference", INFERENCE_PORTABLE),
    ("anyview", ANYVIEW_PORTABLE),
    ("ui", UI_PORTABLE),
];

// ---- cross builds --------------------------------------------------------------------------

/// What a failed cross build means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lenient {
    /// Any failure fails the check.
    No,
    /// A cc-rs error (a dependency's build script found no C cross toolchain) is "not verified
    /// here", never "not portable"; any other error fails.
    MissingToolchain,
    /// A failure is a note: the crate needs the target's C toolchain (`SQLCipher`, `OpenSSL`).
    Note,
}

/// One package built for a target.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CrossBuild {
    package: &'static str,
    features: Vec<&'static str>,
    no_default_features: bool,
    lenient: Lenient,
}

/// The cross builds of one domain.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CrossSet {
    domain: &'static str,
    targets: &'static [&'static str],
    builds: Vec<CrossBuild>,
    /// Set `CC_<target>=clang` when clang is on PATH: a C dependency (cedar-policy reaches `psm`)
    /// assembles a file for the target, and clang does that for any target.
    clang: bool,
}

/// The client API is the one an app on macOS or Windows links, so it must compile there: the
/// Linux-only code (a memfd, a bus) is in account-client-dbus.
const ACCOUNTS_CROSS: &[&str] = &["account-client", "account-sheet", "account-wire"];
const ACCOUNTS_TARGETS: &[&str] = &[
    "x86_64-pc-windows-msvc",
    "x86_64-pc-windows-gnu",
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
];
const ACTIONS_TARGETS: &[&str] = &[
    "x86_64-apple-darwin",
    "x86_64-pc-windows-gnu",
    "x86_64-pc-windows-msvc",
    "aarch64-apple-darwin",
];
const MEMORY_TARGETS: &[&str] = &[
    "x86_64-apple-darwin",
    "x86_64-pc-windows-gnu",
    "x86_64-pc-windows-msvc",
];

/// Memory crates without C dependencies: they check on every target. The `SQLCipher` ones need that
/// target's C toolchain (and OpenSSL's cross build), which a Linux box lacks, so a failure there
/// is only noted.
const MEMORY_PURE_RUST: &[&str] = &[
    "memory-core",
    "memory-seal",
    "memory-store",
    "memory-files",
    "memory-settings",
    "memory-watch",
];

/// blake3 builds C and assembly SIMD for `x86_64` targets, which needs the target's C compiler too;
/// its `pure` feature (through each crate's `pure-hash`) is a plain-Rust build for the check.
const MEMORY_PURE_HASH: &[(&str, &str)] = &[
    ("memory-seal", "pure-hash"),
    ("memory-store", "memory-seal/pure-hash"),
    ("memory-files", "memory-seal/pure-hash"),
    ("memory-watch", "pure-hash"),
];

fn cross_sets() -> Vec<CrossSet> {
    let accounts = CrossSet {
        domain: "accounts",
        targets: ACCOUNTS_TARGETS,
        builds: ACCOUNTS_CROSS
            .iter()
            .map(|package| CrossBuild {
                package,
                features: Vec::new(),
                no_default_features: false,
                lenient: Lenient::No,
            })
            .collect(),
        clang: false,
    };
    let actions = CrossSet {
        domain: "actions",
        targets: ACTIONS_TARGETS,
        builds: ACTIONS_PORTABLE
            .iter()
            .map(|p| CrossBuild {
                package: p.package,
                features: p.features.to_vec(),
                no_default_features: true,
                lenient: Lenient::MissingToolchain,
            })
            .collect(),
        clang: true,
    };
    let memory = CrossSet {
        domain: "memory",
        targets: MEMORY_TARGETS,
        builds: MEMORY_PORTABLE
            .iter()
            .map(|p| {
                let mut features = p.features.to_vec();
                features.extend(
                    MEMORY_PURE_HASH
                        .iter()
                        .filter(|(pkg, _)| *pkg == p.package)
                        .map(|(_, feature)| *feature),
                );
                CrossBuild {
                    package: p.package,
                    features,
                    no_default_features: true,
                    lenient: if MEMORY_PURE_RUST.contains(&p.package) {
                        Lenient::No
                    } else {
                        Lenient::Note
                    },
                }
            })
            .collect(),
        clang: false,
    };
    vec![accounts, actions, memory]
}

// ---- command lines (pure) ------------------------------------------------------------------

/// `cargo hack clippy -p <pkg> --each-feature|--feature-powerset --no-dev-deps --lib --bins -- -D warnings`.
fn clippy_args(package: &str, sweep: Sweep) -> Vec<String> {
    let sweep = match sweep {
        Sweep::Each => "--each-feature",
        Sweep::Powerset => "--feature-powerset",
    };
    [
        "hack",
        "clippy",
        "-p",
        package,
        sweep,
        "--no-dev-deps",
        "--lib",
        "--bins",
        "--",
        "-D",
        "warnings",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// `cargo hack check -p <pkg> --no-default-features --all-targets [--features a,b]`.
fn portable_args(portable: &Portable) -> Vec<String> {
    let mut args: Vec<String> = [
        "hack",
        "check",
        "-p",
        portable.package,
        "--no-default-features",
        "--all-targets",
    ]
    .map(str::to_owned)
    .to_vec();
    if !portable.features.is_empty() {
        args.push("--features".to_owned());
        args.push(portable.features.join(","));
    }
    args
}

/// `cargo check --target <t> -p <pkg> [--no-default-features] [--features a,b]`.
fn cross_args(build: &CrossBuild, target: &str) -> Vec<String> {
    let mut args: Vec<String> = ["check", "--target", target, "-p", build.package]
        .map(str::to_owned)
        .to_vec();
    if build.no_default_features {
        args.push("--no-default-features".to_owned());
    }
    if !build.features.is_empty() {
        args.push("--features".to_owned());
        args.push(build.features.join(","));
    }
    args
}

/// The `CC_<target>` variable name cc-rs reads: the triple with `-` as `_`.
fn cc_var(target: &str) -> String {
    format!("CC_{}", target.replace('-', "_"))
}

/// What one cross build came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Ok,
    /// Not verified here: the C cross toolchain is missing.
    Unverified,
    /// Failed, but this crate needs the target's C toolchain; only noted.
    Note,
    Fail,
}

fn classify(lenient: Lenient, success: bool, output: &str) -> Outcome {
    if success {
        return Outcome::Ok;
    }
    match lenient {
        Lenient::MissingToolchain if output.contains("error occurred in cc-rs") => {
            Outcome::Unverified
        }
        Lenient::No | Lenient::MissingToolchain => Outcome::Fail,
        Lenient::Note => Outcome::Note,
    }
}

/// Every package named by any table.
fn all_packages() -> BTreeSet<&'static str> {
    let mut all = BTreeSet::new();
    for tables in [ACCOUNTS_CLIPPY, INFERENCE_CLIPPY, SYNC_CLIPPY] {
        all.extend(tables.iter().map(|(pkg, _)| *pkg));
    }
    for (_, list) in PORTABLE {
        all.extend(list.iter().map(|p| p.package));
    }
    for set in cross_sets() {
        all.extend(set.builds.iter().map(|b| b.package));
    }
    all
}

// ---- running -------------------------------------------------------------------------------

/// Runs the checks; `cross` adds the cross-target builds. `Ok(true)` is GREEN.
pub fn run(cross: bool) -> Result<bool> {
    let installed = Command::new("cargo")
        .args(["hack", "--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if !installed {
        eprintln!(
            "{SELF_PATH}:1: features: cargo-hack is not installed (cargo install cargo-hack --locked)"
        );
        return Ok(finish("check features", false));
    }
    let ws = Workspace::load()?;
    let mut green = true;
    for package in all_packages() {
        if !ws.dirs.contains_key(package) {
            eprintln!("{SELF_PATH}:1: features: `{package}` is not a workspace member");
            green = false;
        }
    }
    if !green {
        return Ok(finish("check features", false));
    }
    let manifest = |package: &str| format!("{}/Cargo.toml", ws.dirs[package].display());
    let fail = |package: &str, what: &str| {
        eprintln!("{}:1: features: {package}: {what}", manifest(package));
    };

    for (package, sweep) in ACCOUNTS_CLIPPY
        .iter()
        .chain(INFERENCE_CLIPPY)
        .chain(SYNC_CLIPPY)
    {
        let args = clippy_args(package, *sweep);
        if !run_logged(&ws.root, Command::new("cargo").args(&args))? {
            fail(package, &format!("`cargo {}` failed", args.join(" ")));
            green = false;
        }
    }
    for (domain, list) in PORTABLE {
        for portable in *list {
            let args = portable_args(portable);
            if !run_logged(&ws.root, Command::new("cargo").args(&args))? {
                fail(
                    portable.package,
                    &format!("{domain} portable build failed: `cargo {}`", args.join(" ")),
                );
                green = false;
            }
        }
    }
    if cross {
        green &= run_cross(&ws, &fail)?;
    }
    Ok(finish("check features", green))
}

/// The cross builds. Output is captured so a cc-rs failure can be told from a real one; a failed
/// build prints its last lines.
fn run_cross(ws: &Workspace, fail: &dyn Fn(&str, &str)) -> Result<bool> {
    let installed = installed_targets();
    let clang = Command::new("clang")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    let mut green = true;
    for set in cross_sets() {
        for target in set.targets {
            if !installed.contains(*target) {
                println!(
                    "xtask: features: skip {target} for {}: not installed (rustup target add {target})",
                    set.domain
                );
                continue;
            }
            for build in &set.builds {
                let args = cross_args(build, target);
                let mut cmd = Command::new("cargo");
                cmd.args(&args).current_dir(&ws.root);
                if set.clang && clang {
                    cmd.env(cc_var(target), "clang");
                }
                println!("xtask: $ {cmd:?}");
                let out: Output = cmd.output().context("could not start cargo")?;
                let text = format!(
                    "{}{}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                match classify(build.lenient, out.status.success(), &text) {
                    Outcome::Ok => {}
                    Outcome::Unverified => println!(
                        "xtask: features: {} on {target} not verified: a C cross toolchain is missing",
                        build.package
                    ),
                    Outcome::Note => println!(
                        "xtask: features: {} on {target} not checked: it needs the target's C toolchain",
                        build.package
                    ),
                    Outcome::Fail => {
                        let lines: Vec<&str> = text.lines().collect();
                        let tail = lines.len().saturating_sub(40);
                        for line in &lines[tail..] {
                            eprintln!("{line}");
                        }
                        fail(
                            build.package,
                            &format!("does not build for {target}: `cargo {}`", args.join(" ")),
                        );
                        green = false;
                    }
                }
            }
        }
    }
    Ok(green)
}

/// The targets rustup has installed; empty when rustup does not answer.
fn installed_targets() -> BTreeSet<String> {
    Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map_or_default(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(|l| l.trim().to_owned())
                .collect()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clippy_each_feature() {
        assert_eq!(
            clippy_args("http-core", Sweep::Each),
            [
                "hack",
                "clippy",
                "-p",
                "http-core",
                "--each-feature",
                "--no-dev-deps",
                "--lib",
                "--bins",
                "--",
                "-D",
                "warnings"
            ]
        );
    }

    #[test]
    fn clippy_powerset() {
        let args = clippy_args("inference-client", Sweep::Powerset);
        assert!(args.contains(&"--feature-powerset".to_owned()));
        assert!(!args.contains(&"--each-feature".to_owned()));
    }

    #[test]
    fn portable_without_and_with_features() {
        assert_eq!(
            portable_args(&bare("memory-core")),
            [
                "hack",
                "check",
                "-p",
                "memory-core",
                "--no-default-features",
                "--all-targets"
            ]
        );
        let client = MEMORY_PORTABLE
            .iter()
            .find(|p| p.package == "memory-client")
            .unwrap();
        assert_eq!(&portable_args(client)[6..], ["--features", "in_process"]);
    }

    #[test]
    fn cross_line_carries_pure_hash_features() {
        let sets = cross_sets();
        let memory = sets.iter().find(|s| s.domain == "memory").unwrap();
        let store = memory
            .builds
            .iter()
            .find(|b| b.package == "memory-store")
            .unwrap();
        assert_eq!(
            cross_args(store, "x86_64-apple-darwin"),
            [
                "check",
                "--target",
                "x86_64-apple-darwin",
                "-p",
                "memory-store",
                "--no-default-features",
                "--features",
                "memory-seal/pure-hash"
            ]
        );
        let client = memory
            .builds
            .iter()
            .find(|b| b.package == "memory-client")
            .unwrap();
        assert_eq!(client.features, ["in_process"]);
        assert_eq!(client.lenient, Lenient::Note);
        assert_eq!(store.lenient, Lenient::No);
    }

    #[test]
    fn accounts_cross_keeps_default_features() {
        let sets = cross_sets();
        let accounts = sets.iter().find(|s| s.domain == "accounts").unwrap();
        assert_eq!(
            cross_args(&accounts.builds[0], "x86_64-pc-windows-msvc"),
            [
                "check",
                "--target",
                "x86_64-pc-windows-msvc",
                "-p",
                "account-client"
            ]
        );
    }

    #[test]
    fn cc_variable_name() {
        assert_eq!(cc_var("x86_64-apple-darwin"), "CC_x86_64_apple_darwin");
    }

    #[test]
    fn classification() {
        assert_eq!(classify(Lenient::No, true, ""), Outcome::Ok);
        assert_eq!(classify(Lenient::No, false, "error[E0432]"), Outcome::Fail);
        assert_eq!(
            classify(
                Lenient::MissingToolchain,
                false,
                "error occurred in cc-rs: no cc"
            ),
            Outcome::Unverified
        );
        assert_eq!(
            classify(Lenient::MissingToolchain, false, "error[E0432]"),
            Outcome::Fail
        );
        assert_eq!(classify(Lenient::Note, false, "anything"), Outcome::Note);
    }

    #[test]
    fn tables_name_no_package_twice_in_clippy() {
        let mut seen = BTreeSet::new();
        for (pkg, _) in ACCOUNTS_CLIPPY
            .iter()
            .chain(INFERENCE_CLIPPY)
            .chain(SYNC_CLIPPY)
        {
            assert!(seen.insert(*pkg), "{pkg} listed twice");
        }
    }
}
