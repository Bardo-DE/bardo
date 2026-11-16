//! The bardo-shell daemon and command line: `bardo-shell run` is the daemon; every other subcommand is a client
//! that sends one request and prints the reply.
//!
//! The implementation lives in the `bardo-shell` library (`lib.rs`) so `crates/shell/shell-app/tests/**` can
//! exercise it; this binary is a thin wrapper.

use std::process::ExitCode;

use shell_app::cli;

fn main() -> ExitCode {
    // Temporary: delete with the old-folders crate in the release after everyone has started once.
    shell_app::pre_bardo::move_from_env();
    let cli = match cli::parse_args() {
        Ok(cli) => cli,
        Err(code) => return code,
    };
    if let Some(dir) = &cli.write_schema {
        return match shell_settings::schema().write_to(dir) {
            Ok(path) => shell_app::cli::stdout::print(&format!("{}\n", path.display())).exit_code(),
            Err(error) => {
                eprintln!(
                    "bardo-shell: writing the schema to {}: {error}",
                    dir.display()
                );
                ExitCode::FAILURE
            }
        };
    }
    if let Some(dir) = &cli.write_selectors {
        return match shell_app::cli::selectors::write_to(dir) {
            Ok(path) => shell_app::cli::stdout::print(&format!("{}\n", path.display())).exit_code(),
            Err(error) => {
                eprintln!(
                    "bardo-shell: writing the style parts to {}: {error}",
                    dir.display()
                );
                ExitCode::FAILURE
            }
        };
    }
    match cli.command {
        Some(cli::Command::Run(args)) => shell_app::daemon::run(&args),
        Some(cli::Command::AuthCheck) => shell_app::cli::auth_check::main(),
        Some(cli::Command::Style { action }) => shell_app::cli::style::main(&action),
        Some(cli::Command::Launch(args)) => shell_app::cli::launch::main(&args),
        Some(command) => match cli::request(&command) {
            Some(request) => shell_app::cli::client::main(&request),
            None => ExitCode::SUCCESS,
        },
        None => {
            eprintln!("bardo-shell: a subcommand is required (bardo-shell --help)");
            ExitCode::from(cli::USAGE)
        }
    }
}
