//! `cargo xtask`: the repository's one entry point for checks and tasks.
//!
//! This file only parses arguments and dispatches. Loading the workspace is
//! in [`workspace`], pure decisions are in [`affected`], and running commands
//! is in [`tasks`].

mod affected;
mod tasks;
mod workspace;

use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use lexopt::prelude::*;

const USAGE: &str = "\
usage: cargo xtask <command>

commands:
  gate [--base <rev>] [--all]   fmt, clippy and tests on the affected set
  check deny                    cargo deny check (needs cargo-deny)
  check test-modules            every tests/it file is a module of its main.rs
  check legacy [<repo>]         run the old per-repo check-*.sh scripts unchanged
  install [<part>...]           build and install the desktop (every part, or just these)
  install --build [<part>...]   build as you, install nothing
  install --check               what this machine lacks, with the line that installs it
  install --parts               every part, in install order
";

fn main() -> ExitCode {
    match run() {
        Ok(green) => ExitCode::from(u8::from(!green)),
        Err(err) => {
            eprintln!("xtask: error: {err:#}");
            ExitCode::from(2)
        }
    }
}

/// Parses the command line and runs the chosen command. `Ok(true)` is GREEN.
fn run() -> Result<bool> {
    let mut parser = lexopt::Parser::from_env();
    let command = match parser.next()? {
        Some(Value(v)) => v.string()?,
        Some(Long("help") | Short('h')) | None => {
            print!("{USAGE}");
            return Ok(true);
        }
        Some(other) => return Err(other.unexpected().into()),
    };
    match command.as_str() {
        "gate" => {
            let mut base = None;
            let mut all = false;
            while let Some(arg) = parser.next()? {
                match arg {
                    Long("base") => base = Some(parser.value()?.string()?),
                    Long("all") => all = true,
                    other => return Err(other.unexpected().into()),
                }
            }
            tasks::gate::run(base.as_deref(), all)
        }
        "install" => {
            let mut request = tasks::install::Request::default();
            while let Some(arg) = parser.next()? {
                match arg {
                    Long("build") => request.build = true,
                    Long("check") => request.check = true,
                    Long("parts") => request.parts = true,
                    Value(v) => request.names.push(v.string()?),
                    other => return Err(other.unexpected().into()),
                }
            }
            tasks::install::run(&request)
        }
        "check" => {
            let which = parser
                .value()
                .context("`check` needs a name: deny, test-modules or legacy")?
                .string()?;
            match which.as_str() {
                "deny" => {
                    reject_extra(&mut parser)?;
                    tasks::deny::run()
                }
                "test-modules" => {
                    reject_extra(&mut parser)?;
                    tasks::test_modules::run()
                }
                "legacy" => {
                    let mut only = None;
                    while let Some(arg) = parser.next()? {
                        match arg {
                            Value(v) if only.is_none() => only = Some(v.string()?),
                            other => return Err(other.unexpected().into()),
                        }
                    }
                    tasks::legacy::run(only.as_deref())
                }
                other => bail!("unknown check `{other}`\n{USAGE}"),
            }
        }
        other => bail!("unknown command `{other}`\n{USAGE}"),
    }
}

/// Fails on any argument left over after a command took what it needs.
fn reject_extra(parser: &mut lexopt::Parser) -> Result<()> {
    match parser.next()? {
        None => Ok(()),
        Some(arg) => Err(arg.unexpected().into()),
    }
}
