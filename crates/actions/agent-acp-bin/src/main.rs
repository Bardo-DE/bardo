//! docket-acp: the ACP server edge as a process. An editor launches it and speaks ACP on its
//! stdin and stdout; logs go to stderr only. It refuses to serve unless the setting
//! `agent.acp.expose` is on. Exits 2 when that is off or the command line is bad, 1 for any other
//! stop.

use std::process::ExitCode;

const USAGE: &str =
    "usage: docket-acp   (no arguments: the router knows this process by its bus name)";

fn main() -> ExitCode {
    let env = |key: &str| std::env::var(key).ok();
    if std::env::args().nth(1).is_some() {
        eprintln!("docket-acp: {USAGE}");
        return ExitCode::from(2);
    }
    action_log::init("docket-acp");
    if !agent_acp_server::exposed_in(&env) {
        eprintln!("docket-acp: agent.acp.expose is off: docket-acp refuses to serve");
        return ExitCode::from(2);
    }
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(why) => {
            eprintln!("docket-acp: no runtime: {why}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(agent_acp_bin::serve(action_dbus::InferWatch::off(), &env)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("docket-acp: {why}");
            ExitCode::FAILURE
        }
    }
}
