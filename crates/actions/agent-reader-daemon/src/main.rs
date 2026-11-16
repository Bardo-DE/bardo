//! readerd: the quarantined reader, serving `org.bardo.Reader1` on the session bus until the bus
//! closes.

use std::process::ExitCode;

/// Temporary: delete with the old-folders crate in the release after everyone has started once.
///
/// Moves the person's callers file of the earlier release (`quire/readerd-callers.toml`) into
/// `bardo/actions`.
fn move_pre_bardo_folders(env: &impl Fn(&str) -> Option<String>) {
    action_dbus::pre_bardo::move_callers(
        "bardo-readerd",
        "readerd-callers.toml",
        "readerd-callers.toml",
        env,
    );
}

fn main() -> ExitCode {
    action_log::init("readerd");
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(why) => {
            eprintln!("readerd: no runtime: {why}");
            return ExitCode::FAILURE;
        }
    };
    let env = |key: &str| std::env::var(key).ok();
    move_pre_bardo_folders(&env);
    match runtime.block_on(agent_reader_daemon::run(&env)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            tracing::error!("stopped: {why}");
            eprintln!("readerd: {why}");
            ExitCode::FAILURE
        }
    }
}
