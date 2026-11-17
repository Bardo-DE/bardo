//! Tasks: the shell of xtask. Everything here runs commands or touches files.

pub mod deny;
pub mod domain_order;
pub mod gate;
pub mod install;
pub mod legacy;
pub mod renderer_edges;
pub mod shear;
pub mod test_modules;
pub mod workspace_deps;

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

/// Prints `command` and runs it in `dir` with inherited output. `Ok(true)` if it exited 0.
pub fn run_logged(dir: &Path, command: &mut Command) -> Result<bool> {
    println!("xtask: $ {command:?}");
    let status = command.current_dir(dir).status().with_context(|| {
        format!(
            "could not start {}",
            command.get_program().to_string_lossy()
        )
    })?;
    Ok(status.success())
}

/// [`run_logged`] with a deadline: the command runs in its own process group, and when it is still
/// running after `deadline` the whole group is stopped (TERM, then KILL after 10 s) and the step
/// counts as failed. A hung test or build can then never hold a gate open.
pub fn run_logged_within(dir: &Path, command: &mut Command, deadline: Duration) -> Result<bool> {
    use std::os::unix::process::CommandExt;
    println!(
        "xtask: $ {command:?}  (deadline {} min)",
        deadline.as_secs() / 60
    );
    let mut child = command
        .current_dir(dir)
        .process_group(0)
        .spawn()
        .with_context(|| {
            format!(
                "could not start {}",
                command.get_program().to_string_lossy()
            )
        })?;
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status.success());
        }
        if started.elapsed() > deadline {
            println!(
                "xtask: still running after {} min: stopping it and everything it started",
                deadline.as_secs() / 60
            );
            let group = format!("-{}", child.id());
            let _ = Command::new("kill").args(["-TERM", "--", &group]).status();
            let grace = Instant::now();
            while child.try_wait()?.is_none() && grace.elapsed() < Duration::from_secs(10) {
                std::thread::sleep(Duration::from_millis(200));
            }
            let _ = Command::new("kill").args(["-KILL", "--", &group]).status();
            let _ = child.wait();
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(500));
    }
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
