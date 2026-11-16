//! companiond: serves `org.bardo.Companion1` on the session bus until the bus closes.

use std::process::ExitCode;

/// Temporary: delete with the old-folders crate in the release after everyone has started once.
///
/// Moves the person's files of the earlier release (`quire/companiond.toml` and its callers file)
/// into `bardo/actions`.
fn move_pre_bardo_folders(env: &impl Fn(&str) -> Option<String>) {
    use action_dbus::pre_bardo::{config_base, move_callers, move_file};
    const WHO: &str = "bardo-companiond";
    let config = config_base(env);
    move_file(
        WHO,
        &config.join("quire").join("companiond.toml"),
        &config.join("bardo").join("actions").join("companiond.toml"),
    );
    move_callers(
        WHO,
        "companiond-callers.toml",
        "companiond-callers.toml",
        env,
    );
}

fn main() -> ExitCode {
    action_log::init("companiond");
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(why) => {
            eprintln!("companiond: no runtime: {why}");
            return ExitCode::FAILURE;
        }
    };
    let env = |key: &str| std::env::var(key).ok();
    move_pre_bardo_folders(&env);
    match runtime.block_on(companion_daemon::run(&env)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("companiond: {why}");
            ExitCode::FAILURE
        }
    }
}
