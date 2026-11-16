//! bardo-agent-acp: the ACP server edge as a process. An editor launches it and speaks ACP on its
//! stdin and stdout; logs go to stderr only. It refuses to serve unless the setting
//! `agent.acp.expose` is on. Exits 2 when that is off or the command line is bad, 1 for any other
//! stop.

use std::process::ExitCode;

const USAGE: &str =
    "usage: bardo-agent-acp   (no arguments: the router knows this process by its bus name)";

fn main() -> ExitCode {
    let env = |key: &str| std::env::var(key).ok();
    if std::env::args().nth(1).is_some() {
        eprintln!("bardo-agent-acp: {USAGE}");
        return ExitCode::from(2);
    }
    action_log::init("bardo-agent-acp");
    if !agent_acp_server::exposed_in(&env) {
        eprintln!("bardo-agent-acp: agent.acp.expose is off: bardo-agent-acp refuses to serve");
        return ExitCode::from(2);
    }
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(why) => {
            eprintln!("bardo-agent-acp: no runtime: {why}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(agent_acp_bin::serve(action_dbus::InferWatch::off(), &env)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("bardo-agent-acp: {why}");
            ExitCode::FAILURE
        }
    }
}
