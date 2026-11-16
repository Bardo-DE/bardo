//! The `bardo-compositor` binary: arguments, the environment and an exit code around [`compositor_app::run`].

#[cfg(feature = "debug-inject")]
mod debug_peer_units;

use compositor_app::cli::{self, Command};
use compositor_app::environment::run_config;
use std::process::ExitCode;

fn main() -> ExitCode {
    let filter = std::env::var("BARDO_COMPOSITOR_LOG")
        .unwrap_or_else(|_| compositor_app::log::DEFAULT_FILTER.to_owned());
    compositor_app::log::init(&filter);
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::parse_args(&args) {
        Ok(Command::Help) => {
            print!("{}", cli::USAGE);
            ExitCode::SUCCESS
        }
        Ok(Command::WriteSchema(dir)) => match compositor_app::config::write_schema(&dir) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("bardo-compositor: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(Command::Run(start)) => {
            let config = run_config(*start);
            #[cfg(feature = "debug-inject")]
            let config = match debug_peer_units::from_env() {
                Ok(table) => config.with_debug_peer_units(table),
                Err(error) => {
                    eprintln!("bardo-compositor: {error}");
                    return ExitCode::from(64);
                }
            };
            match compositor_app::run(config) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("bardo-compositor: {error}");
                    ExitCode::from(error.exit_code())
                }
            }
        }
        Err(error) => {
            eprintln!("bardo-compositor: {error}\n{}", cli::USAGE);
            ExitCode::from(64)
        }
    }
}
