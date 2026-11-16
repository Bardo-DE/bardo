//! `agent-bringup up|down|status`: the agent services on a demo shell's private bus. The logic
//! is `action_live::bringup`; see `docs/demo-bringup.md`.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exe = std::env::current_exe().unwrap_or_default();
    action_live::bringup::run_cli(&args, &exe, &|key: &str| std::env::var(key).ok())
}
