//! intentd as its own process: the packaged main, under a name of the acceptance harness.

use std::process::ExitCode;

fn main() -> ExitCode {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(why) => {
            eprintln!("intentd: no runtime: {why}");
            return ExitCode::FAILURE;
        }
    };
    let env = |key: &str| std::env::var(key).ok();
    let seams = action_daemon::HarnessSeams {
        proc_root: action_accept::seams::proc_root(
            &env,
            action_accept::seams::INTENTD_PROC_ROOT_VAR,
        ),
        system_callers: action_accept::seams::system_callers(
            &env,
            action_accept::seams::INTENTD_SYSTEM_CALLERS_VAR,
        ),
        trace: action_accept::seams::trace(&env),
        floor: action_accept::seams::system_callers(&env, action_accept::seams::INTENTD_FLOOR_VAR),
        admin_permissions: action_accept::seams::system_callers(
            &env,
            action_accept::seams::INTENTD_PERMISSIONS_VAR,
        ),
    };
    match runtime.block_on(action_daemon::run_with(seams, &env)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("intentd: {why}");
            ExitCode::FAILURE
        }
    }
}
