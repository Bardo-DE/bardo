//! Builds the three daemons the acceptance runs as the packaged binaries: the accounts daemon (`accountd` (it
//! holds the AI keys), steward's `inferd` and almanac's `memoryd`. `accountd` and `memoryd` are built with
//! `test-keys`, because the private bus has no Secret Service, and all three with `test-proc-root`,
//! so callers are named from a fake proc root. Each is built from its own folder, into a target
//! directory inside this build's (`<target>/accept-siblings/<repo>`), so the nextest archive's
//! target directory, which the jail binds, holds them and `cargo clean` removes them. The three
//! repos are folders of the one workspace this package sits in, so each is built from its folder
//! (`ACCEPT_<REPO>_DIR` names another checkout to build instead).
//!
//! The paths reach the tests as `BARDO_ACCEPT_ACCOUNTD`, `BARDO_ACCEPT_INFERD` and `BARDO_ACCEPT_MEMORYD`. The nested cargo is the one
//! the outer cargo runs as (`$CARGO`, not the build-slot shim: the outer build already holds a
//! slot), with every cargo variable of this build scrubbed so the two do not share flags, a
//! jobserver or a target directory.

use std::path::{Path, PathBuf};
use std::process::Command;

struct Sibling {
    /// The package's folder under `crates/`.
    dir: &'static str,
    /// Its binary, which the acceptance runs.
    bin: &'static str,
    features: &'static [&'static str],
}

const SIBLINGS: [Sibling; 3] = [
    Sibling {
        dir: "accounts/account-daemon",
        bin: "bardo-accountd",
        features: &["test-proc-root", "test-keys"],
    },
    Sibling {
        dir: "inference/inference-daemon",
        bin: "bardo-inferd",
        features: &["test-proc-root"],
    },
    Sibling {
        dir: "memory/memory-daemon",
        bin: "bardo-memoryd",
        features: &["test-keys", "test-proc-root"],
    },
];

fn var(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set"))
}

/// The root of the workspace this package is in: the nearest folder above `start` whose
/// `Cargo.toml` has a `[workspace]` table.
fn workspace_root(start: &Path) -> PathBuf {
    start
        .ancestors()
        .find(|dir| {
            std::fs::read_to_string(dir.join("Cargo.toml"))
                .is_ok_and(|text| text.lines().any(|line| line.trim() == "[workspace]"))
        })
        .unwrap_or_else(|| panic!("no workspace root above {}", start.display()))
        .to_path_buf()
}

fn build(sibling: &Sibling, root: &Path, target: &Path) -> PathBuf {
    let source = root.join("crates").join(sibling.dir);
    println!("cargo:rerun-if-changed={}", source.display());
    let mut command = Command::new(var("CARGO"));
    command.args(["install", "--locked", "--quiet", "--force", "--debug"]);
    command.args(["--root"]).arg(target);
    command.args(["--target-dir"]).arg(target.join("build"));
    command.args(["--path"]).arg(&source);
    command.args(["--bin", sibling.bin]);
    if !sibling.features.is_empty() {
        command.args(["--features", &sibling.features.join(",")]);
    }
    std::env::vars()
        .map(|(name, _)| name)
        .filter(|name| {
            (name.starts_with("CARGO_") && name != "CARGO_HOME")
                // A `cargo clippy` run lints through the workspace wrapper: these siblings are
                // built to run, not to be linted again under another feature set.
                || name.starts_with("CLIPPY")
                || name.starts_with("__CLIPPY")
        })
        .chain(
            [
                "MAKEFLAGS",
                "MFLAGS",
                "RUSTFLAGS",
                "RUSTDOCFLAGS",
                "RUSTC_WORKSPACE_WRAPPER",
            ]
            .map(str::to_owned),
        )
        .for_each(|name| {
            command.env_remove(name);
        });
    command.env("CARGO_TARGET_DIR", target);
    let status = command
        .status()
        .unwrap_or_else(|e| panic!("cargo for {}: {e}", sibling.dir));
    assert!(
        status.success(),
        "building {} failed ({status})",
        sibling.dir
    );
    target.join("bin").join(sibling.bin)
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out = PathBuf::from(var("OUT_DIR"));
    // <target>/<profile>/build/<package>-<hash>/out
    let target = out
        .ancestors()
        .nth(4)
        .expect("OUT_DIR is inside a target directory")
        .join("accept-siblings");
    let root = workspace_root(Path::new(&var("CARGO_MANIFEST_DIR")));
    for sibling in &SIBLINGS {
        let binary = build(sibling, &root, &target.join(sibling.bin));
        let name = sibling.bin.trim_start_matches("bardo-").to_uppercase();
        println!("cargo:rustc-env=BARDO_ACCEPT_{name}={}", binary.display());
    }
}
