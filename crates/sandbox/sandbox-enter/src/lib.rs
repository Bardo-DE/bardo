//! The exec stage inside the sandbox: the one program bubblewrap starts first when an agent runs
//! with Landlock. It applies a Landlock ruleset to itself (`apply`), reports the level it reached
//! on standard error in one fixed line (`bardo-sandbox-enter: landlock=<word>`), and then replaces
//! itself with the agent (`enter_main`). Landlock is a second layer behind bubblewrap and the
//! syscall filter: bubblewrap decides what is mounted, Landlock decides what the agent may do
//! with it.
//!
//! The words the stage is given (`EnterPlan`) and the report line (`LandlockLevel`) are plain data
//! in `sandbox-core`, built by the same code that builds the bubblewrap arguments. This crate is
//! only the Landlock calls and the program, so what starts a sandbox links no Landlock library.
//! Linux only.

mod apply;

pub use apply::apply;

use sandbox_core::EnterPlan;

/// The exec stage's `main`, as a function so any binary can be it.
///
/// Its arguments are
/// `NAME [--read P]... [--write P]... [--device P]... NETWORK -- PROGRAM [ARGS...]`.
/// Exits 2 on words it refuses, 3 when Landlock fails in a way that is not "not available", and
/// 127 when the program cannot be started.
pub fn enter_main(name: &str) -> std::process::ExitCode {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, ExitCode};
    let Ok((plan, program)) = EnterPlan::parse(std::env::args().skip(1)) else {
        eprintln!(
            "usage: {name} [--read PATH]... [--write PATH]... [--device PATH]... \
             (--net-unfiltered | --net-none | --net-connect PORT) -- PROGRAM [ARGS]"
        );
        return ExitCode::from(2);
    };
    let level = match apply(&plan) {
        Ok(level) => level,
        Err(error) => {
            eprintln!("{name}: cannot apply Landlock: {error}");
            return ExitCode::from(3);
        }
    };
    eprintln!("{}", level.report_line());
    let error = Command::new(&program[0]).args(&program[1..]).exec();
    eprintln!("{name}: cannot run {}: {error}", program[0]);
    ExitCode::from(127)
}
