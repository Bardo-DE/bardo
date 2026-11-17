//! `cargo xtask check shear`: no crate declares a dependency it does not use, and none lists
//! a test-only dependency as a normal one.

use std::process::{Command, Stdio};

use anyhow::Result;

use super::{finish, run_logged};
use crate::workspace::Workspace;

/// Runs `cargo shear` at the workspace root; FAIL with a clear message if cargo-shear is missing.
pub fn run() -> Result<bool> {
    let installed = Command::new("cargo")
        .args(["shear", "--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if !installed {
        eprintln!(
            "Cargo.toml:1: shear-missing: cargo-shear is not installed (cargo install cargo-shear --locked)"
        );
        return Ok(finish("check shear", false));
    }
    let ws = Workspace::load()?;
    let green = run_logged(&ws.root, Command::new("cargo").args(["shear"]))?;
    Ok(finish("check shear", green))
}
