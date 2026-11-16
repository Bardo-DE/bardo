//! readerd: the quarantined reader, serving `org.bardo.Reader1` on the session bus until the bus
//! closes.

use std::process::ExitCode;

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
    match runtime.block_on(agent_reader_daemon::run(&env)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            tracing::error!("stopped: {why}");
            eprintln!("readerd: {why}");
            ExitCode::FAILURE
        }
    }
}
