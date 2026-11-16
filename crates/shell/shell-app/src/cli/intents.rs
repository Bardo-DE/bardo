//! `bardo-shell intents status|revoke|resume` and its pure mapping to a request.

use clap::Subcommand;
use shell_ipc::request::Request;
use shell_ipc::status::StatusRequest;
use shell_model::intents::terminal::parse_grant;
use shell_model::intents::{ActionRef, IntentsAsk};

/// `bardo-shell intents ...`.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum IntentsAction {
    /// Whether bardo-shell serves intentd's sheet, what waits on it, and what the terminal may do
    /// without asking.
    Status,
    /// Withdraw one standing terminal grant: `bardo-shell intents revoke org.bardo.Shell:shell.dock.pin`.
    Revoke {
        #[arg(value_name = "APP:ACTION", value_parser = parse_grant)]
        grant: ActionRef,
    },
    /// Let the terminal go on after the breaker paused it (too many refusals in a row).
    Resume,
}

/// The request `bardo-shell intents ACTION` sends.
pub fn intents_request(action: &IntentsAction) -> Request {
    match action {
        IntentsAction::Status => Request::Status(StatusRequest::Intents),
        IntentsAction::Revoke { grant } => Request::Intents(IntentsAsk::Revoke(grant.clone())),
        IntentsAction::Resume => Request::Intents(IntentsAsk::Resume),
    }
}
