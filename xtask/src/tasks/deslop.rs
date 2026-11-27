//! `cargo xtask check deslop`: the structure lints of deslop-please (module cycles, tangles, field
//! leaks, shallow modules, wide folders, long files) over the workspace, with the findings recorded in
//! `deslop.allow` let through. A new finding fails; an entry that no longer fires never does, so the
//! list only shrinks as the code is fixed.

use std::process::{Command, Stdio};

use anyhow::Result;

use super::{finish, run_logged};
use crate::workspace::Workspace;

/// The findings recorded on purpose or as debt, at the workspace root.
const ALLOW: &str = "deslop.allow";

/// Runs `deslop-please --allow deslop.allow` at the workspace root; FAIL with a clear message if the
/// tool is missing.
pub fn run() -> Result<bool> {
    let installed = Command::new("deslop-please")
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if !installed {
        eprintln!(
            "{ALLOW}:1: deslop-missing: deslop-please is not installed (cargo install deslop-please --version 0.1.4 --locked); it also needs the toolchain's rust-analyzer component"
        );
        return Ok(finish("check deslop", false));
    }
    let ws = Workspace::load()?;
    let green = run_logged(
        &ws.root,
        Command::new("deslop-please").args(["--allow", ALLOW]),
    )?;
    Ok(finish("check deslop", green))
}
