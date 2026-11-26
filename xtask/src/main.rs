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
  gate [--base <rev>] [--all]   workspace-deps, renderer-edges, layers, lints, product-words, error-names, no-mod-rs, no-prefs, test-modules, shear and deny, then fmt, clippy and tests on the affected set
  check deny                    cargo deny check (needs cargo-deny)
  check features [--cross]      each package built alone, per feature (clippy) and without default features (portable core); --cross adds macOS and Windows targets (needs cargo-hack)
  check test-modules            one test binary per crate (tests/it), every tests/it file a module of its main.rs
  check workspace-deps          outside dependencies are pinned in the root workspace table
  check layers                  the rules in layers.toml: layer order, programs are leaves, reach, edges, source
  check lints                   every crate takes the workspace lints; unsafe and baseline crates as layers.toml marks them
  check product-words           no retired product name in a file name or in the text of any tracked file
  check error-names             a type that implements std::error::Error has a name ending in Error
  check no-mod-rs               no mod.rs file
  check no-prefs                no identifier ending in Prefs
  check renderer-edges          only the renderer crates and the programs depend on the renderer
  check shear                   no crate declares a dependency it does not use (needs cargo-shear)
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
        "check" => check(&mut parser),
        other => bail!("unknown command `{other}`\n{USAGE}"),
    }
}

/// Runs `cargo xtask check <name>`.
fn check(parser: &mut lexopt::Parser) -> Result<bool> {
    let which = parser
        .value()
        .context("`check` needs a name: deny, features, test-modules, workspace-deps, renderer-edges, layers, lints, product-words, error-names, no-mod-rs, no-prefs or shear")?
        .string()?;
    match which.as_str() {
        "deny" => {
            reject_extra(parser)?;
            tasks::deny::run()
        }
        "features" => {
            let mut cross = false;
            while let Some(arg) = parser.next()? {
                match arg {
                    Long("cross") => cross = true,
                    other => return Err(other.unexpected().into()),
                }
            }
            tasks::features::run(cross)
        }
        "test-modules" => {
            reject_extra(parser)?;
            tasks::test_modules::run()
        }
        "workspace-deps" => {
            reject_extra(parser)?;
            tasks::workspace_deps::run()
        }
        "renderer-edges" => {
            reject_extra(parser)?;
            tasks::renderer_edges::run()
        }
        "layers" => {
            reject_extra(parser)?;
            tasks::layers::run()
        }
        "lints" => {
            reject_extra(parser)?;
            tasks::lints::run()
        }
        "product-words" => {
            reject_extra(parser)?;
            tasks::product_words::run()
        }
        "error-names" => {
            reject_extra(parser)?;
            tasks::error_names::run()
        }
        "no-mod-rs" => {
            reject_extra(parser)?;
            tasks::no_mod_rs::run()
        }
        "no-prefs" => {
            reject_extra(parser)?;
            tasks::no_prefs::run()
        }
        "shear" => {
            reject_extra(parser)?;
            tasks::shear::run()
        }
        other => bail!("unknown check `{other}`\n{USAGE}"),
    }
}

/// Fails on any argument left over after a command took what it needs.
fn reject_extra(parser: &mut lexopt::Parser) -> Result<()> {
    match parser.next()? {
        None => Ok(()),
        Some(arg) => Err(arg.unexpected().into()),
    }
}
