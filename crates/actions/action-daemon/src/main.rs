//! actiond: serves `org.bardo.Intents1` on the session bus until the bus closes.

use std::process::ExitCode;

fn main() -> ExitCode {
    action_log::init("actiond");
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(why) => {
            eprintln!("actiond: no runtime: {why}");
            return ExitCode::FAILURE;
        }
    };
    let env = |key: &str| std::env::var(key).ok();
    match runtime.block_on(action_daemon::run(&env)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("actiond: {why}");
            ExitCode::FAILURE
        }
    }
}
