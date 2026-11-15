//! `cargo xtask check deny`.

use std::process::{Command, Stdio};

use anyhow::Result;

use super::{finish, run_logged};
use crate::workspace::Workspace;

/// Runs `cargo deny check` at the workspace root; FAIL with a clear message if cargo-deny is missing.
pub fn run() -> Result<bool> {
    let installed = Command::new("cargo")
        .args(["deny", "--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if !installed {
        eprintln!(
            "deny.toml:1: deny-missing: cargo-deny is not installed (cargo install cargo-deny --locked)"
        );
        return Ok(finish("check deny", false));
    }
    let ws = Workspace::load()?;
    let green = run_logged(&ws.root, Command::new("cargo").args(["deny", "check"]))?;
    Ok(finish("check deny", green))
}
