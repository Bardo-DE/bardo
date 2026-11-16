//! The voiced binary: reads `voiced.toml`, then serves `org.bardo.Voice1` on the session bus over
//! PipeWire until the bus closes.

use agent_voice_daemon::{ConfigError, FileUse, VoicedConfig};
use agent_voice_pipewire::PipeWireDevice;
use clap::Parser;
use std::process::ExitCode;

/// The voice daemon.
#[derive(Debug, Parser)]
#[command(name = "bardo-voiced", about = "The voice daemon (org.bardo.Voice1)")]
struct Args {
    /// Path of voiced.toml.
    #[arg(long, default_value = "/etc/bardo/voiced.toml")]
    config: std::path::PathBuf,
}

/// Why the configuration was not read.
#[derive(Debug, thiserror::Error)]
enum LoadFault {
    #[error("{path}: {source}")]
    Unread {
        path: String,
        source: std::io::Error,
    },
    #[error("{0}")]
    Invalid(#[from] ConfigError),
}

fn load(path: &std::path::Path) -> Result<VoicedConfig, LoadFault> {
    let text = std::fs::read_to_string(path).map_err(|source| LoadFault::Unread {
        path: path.display().to_string(),
        source,
    })?;
    Ok(VoicedConfig::parse(&text)?)
}

/// Temporary: delete with the old-folders crate in the release after everyone has started once.
///
/// Moves the person's callers file of the earlier release (`quire/voiced-callers.toml`) into
/// `bardo/actions`.
fn move_pre_bardo_folders(env: &impl Fn(&str) -> Option<String>) {
    action_dbus::pre_bardo::move_callers(
        "bardo-voiced",
        "voiced-callers.toml",
        "voiced-callers.toml",
        env,
    );
}

fn main() -> ExitCode {
    action_log::init("voiced");
    move_pre_bardo_folders(&|name: &str| std::env::var(name).ok());
    let args = Args::parse();
    let config = match load(&args.config) {
        Ok(config) => config,
        Err(why) => {
            tracing::error!("configuration not read: {why}");
            eprintln!("voiced: {why}");
            return ExitCode::FAILURE;
        }
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(why) => {
            eprintln!("voiced: no runtime: {why}");
            return ExitCode::FAILURE;
        }
    };
    let table = match agent_voice_daemon::read_callers(&|name: &str| std::env::var(name).ok()) {
        Ok(table) => table,
        Err(why) => {
            tracing::error!("callers not read: {why}");
            eprintln!("voiced: {why}");
            return ExitCode::FAILURE;
        }
    };
    let usage = FileUse::sill_default(|name| std::env::var(name).ok());
    tracing::info!("serving");
    match runtime.block_on(agent_voice_daemon::serve(
        config,
        table,
        PipeWireDevice,
        usage,
    )) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            tracing::error!("stopped: {why}");
            eprintln!("voiced: {why}");
            ExitCode::FAILURE
        }
    }
}
