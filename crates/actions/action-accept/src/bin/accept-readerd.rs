//! readerd as its own process: the packaged main, under a name of the acceptance harness.

use std::process::ExitCode;

fn main() -> ExitCode {
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
    let seams = agent_reader_daemon::HarnessSeams {
        proc_root: action_accept::seams::proc_root(
            &env,
            action_accept::seams::READERD_PROC_ROOT_VAR,
        ),
        system_callers: action_accept::seams::system_callers(
            &env,
            action_accept::seams::READERD_SYSTEM_CALLERS_VAR,
        ),
    };
    match runtime.block_on(agent_reader_daemon::run_with(
        action_accept::seams::trace(&env),
        seams,
        &env,
    )) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("readerd: {why}");
            ExitCode::FAILURE
        }
    }
}
