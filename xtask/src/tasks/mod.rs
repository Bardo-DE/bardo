//! Tasks: the shell of xtask. Everything here runs commands or touches files.

pub mod deny;
pub mod gate;
pub mod legacy;

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

/// Prints `command` and runs it in `dir` with inherited output. `Ok(true)` if it exited 0.
pub fn run_logged(dir: &Path, command: &mut Command) -> Result<bool> {
    println!("xtask: $ {command:?}");
    let status = command
        .current_dir(dir)
        .status()
        .with_context(|| format!("could not start {:?}", command.get_program()))?;
    Ok(status.success())
}

/// Runs `git <args>` in `dir` and returns its stdout as lines. `None` if git exits non-zero.
pub fn git_lines(dir: &Path, args: &[&str]) -> Result<Option<Vec<String>>> {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .context("could not start git")?;
    if !out.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(Some(text.lines().map(str::to_owned).collect()))
}

/// Prints the final `xtask: <command> GREEN|FAIL` line and returns `green`.
pub fn finish(command: &str, green: bool) -> bool {
    println!("xtask: {command} {}", if green { "GREEN" } else { "FAIL" });
    green
}
