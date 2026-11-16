//! `bardo-agent PROGRAM [--cwd DIR] [--tty]`, or `bardo-agent --serve`: see
//! `agent_acp_bin::entry::docket_agent`.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    Box::pin(agent_acp_bin::entry::docket_agent(
        std::env::args().skip(1),
        std::env::current_dir().unwrap_or_default(),
        |key: &str| std::env::var(key).ok(),
    ))
    .await
}
