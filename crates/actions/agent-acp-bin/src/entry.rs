//! The program entry points `main` wires: what a binary of this package calls once it has
//! collected its arguments and environment.

use crate::agent::args::{self, Request};
use crate::agent::{self, serve};
use std::path::PathBuf;
use std::process::ExitCode;

/// Temporary: delete with the old-folders crate in the release after everyone has started once.
///
/// Moves the agents and helpers the earlier release kept in `docket/` under the data folder, and
/// the person's callers file of `quire/`, to the `bardo/actions` folders.
fn move_pre_bardo_folders(env: &impl Fn(&str) -> Option<String>) {
    use action_dbus::pre_bardo::{data_base, move_callers, move_dir};
    const WHO: &str = "bardo-agent";
    let data = data_base(env);
    move_dir(
        WHO,
        &data.join("docket"),
        &data.join("bardo").join("actions"),
    );
    move_callers(WHO, "acp-callers.toml", "agent-acp-callers.toml", env);
}

/// `bardo-agent PROGRAM [--cwd DIR] [--tty]`, or `bardo-agent --serve`. `argv` is the command
/// line after the program's name, `here` the working directory and `env` the environment `main`
/// hands in. A bad command line exits the process with clap's usage; a setting that is off or a
/// configuration that is wrong ends with code 2.
pub async fn docket_agent(
    argv: impl IntoIterator<Item = String>,
    here: PathBuf,
    env: impl Fn(&str) -> Option<String> + Send + Sync + 'static,
) -> ExitCode {
    move_pre_bardo_folders(&env);
    let ended = match args::parse(argv, here) {
        Ok(Request::Serve) => serve::run(env).await,
        Ok(Request::Run(args)) => agent::run(args, env).await,
        Err(usage) => usage.exit(),
    };
    match ended {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("bardo-agent: {why}");
            ExitCode::from(2)
        }
    }
}
