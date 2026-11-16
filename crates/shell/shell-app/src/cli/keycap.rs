//! `bardo-shell keycap status|retry|resume` and its pure mapping to a request.

use clap::Subcommand;
use shell_ipc::request::Request;
use shell_ipc::status::StatusRequest;
use shell_model::keycap::KeycapAsk;

/// `bardo-shell keycap ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum KeycapAction {
    /// Whether keycap runs, and what it says about its mode and the keyboards.
    Status,
    /// Start keycap again after it stopped.
    Retry,
    /// Turn the layout back on after the escape chord paused it.
    Resume,
}

/// The request `bardo-shell keycap ACTION` sends.
pub fn keycap_request(action: KeycapAction) -> Request {
    match action {
        KeycapAction::Status => Request::Status(StatusRequest::Keycap),
        KeycapAction::Retry => Request::Keycap(KeycapAsk::Retry),
        KeycapAction::Resume => Request::Keycap(KeycapAsk::Resume),
    }
}
