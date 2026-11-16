//! The command-line client: connect to the daemon (starting it if needed), send one request, print
//! the reply.

use std::io::BufReader;
use std::process::ExitCode;
use std::time::Duration;

use super::stdout;
use shell_ipc::error::IpcError;
use shell_ipc::line::{encode, read_message, write_message};
use shell_ipc::request::Request;
use shell_ipc::response::Response;

/// How long a client waits for a daemon it started.
pub const START_TIMEOUT: Duration = Duration::from_secs(3);

/// Send `request`, print the reply, exit 0 on success.
#[must_use]
pub fn main(request: &Request) -> ExitCode {
    match send(request) {
        Ok(Response::Failed(failure)) => {
            eprintln!("bardo-shell: {}", failure.message);
            ExitCode::FAILURE
        }
        Ok(response) => match encode(&response) {
            Ok(line) => stdout::print(&line).exit_code(),
            Err(error) => {
                eprintln!("bardo-shell: encoding the reply: {error}");
                ExitCode::FAILURE
            }
        },
        Err(ClientError::NotRunning) => {
            eprintln!("bardo-shell: no daemon is running");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("bardo-shell: {error}");
            ExitCode::FAILURE
        }
    }
}

/// One request's reply. `Quit`, `Ping` and `Launch` never start a daemon: a launch asked of a
/// shell that was not running would time the shell's start, not the launch.
pub fn send(request: &Request) -> Result<Response, ClientError> {
    let agent = service_locator::Agent::new("bardo-shell")?;
    let mut stream = match request {
        Request::Ping | Request::Quit | Request::Launch(_) => {
            agent.connect()?.ok_or(ClientError::NotRunning)?
        }
        _ => agent.connect_or_start(|| service_locator::spawn(&["run"]), START_TIMEOUT)?,
    };
    write_message(&mut stream, request)?;
    let mut reader = BufReader::new(stream);
    Ok(read_message(&mut reader)?)
}

/// Why a request got no reply.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// No daemon, and this request does not start one.
    #[error("no daemon is running")]
    NotRunning,
    /// latchkey could not find or start the daemon.
    #[error("{0}")]
    Agent(#[from] service_locator::Error),
    /// The reply could not be read.
    #[error("{0}")]
    Ipc(#[from] IpcError),
}
