//! bardo-actions-mcp: the MCP edge as a process. Stdio by default (a client starts it), `--socket PATH`
//! for a long-lived socket. Exits 2 for a bad command line, 1 for any other stop.

use std::process::ExitCode;

/// Temporary: delete with the old-folders crate in the release after everyone has started once.
///
/// Moves the person's configuration of the earlier release (`quire/actions-mcp.toml`) into
/// `bardo/actions`.
fn move_pre_bardo_folders(env: &impl Fn(&str) -> Option<String>) {
    use action_dbus::pre_bardo::{config_base, move_file};
    let config = config_base(env);
    move_file(
        "bardo-actions-mcp",
        &config.join("quire").join("actions-mcp.toml"),
        &config
            .join("bardo")
            .join("actions")
            .join("actions-mcp.toml"),
    );
}

fn main() -> ExitCode {
    let args = match action_mcp::Args::parse(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(why) => {
            eprintln!("bardo-actions-mcp: {why}");
            return ExitCode::from(2);
        }
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(why) => {
            eprintln!("bardo-actions-mcp: no runtime: {why}");
            return ExitCode::FAILURE;
        }
    };
    let env = |key: &str| std::env::var(key).ok();
    move_pre_bardo_folders(&env);
    match runtime.block_on(action_mcp::run(args, &env)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("bardo-actions-mcp: {why}");
            ExitCode::FAILURE
        }
    }
}
