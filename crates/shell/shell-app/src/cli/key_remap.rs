//! `bardo-shell key-remap status|retry|resume` and its pure mapping to a request.

use clap::Subcommand;
use shell_ipc::request::Request;
use shell_ipc::status::StatusRequest;
use shell_model::key_remap::KeyRemapAsk;

/// `bardo-shell key-remap ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum KeyRemapAction {
    /// Whether the key remap daemon runs, and what it says about its mode and the keyboards.
    Status,
    /// Start the key remap daemon again after it stopped.
    Retry,
    /// Turn the layout back on after the escape chord paused it.
    Resume,
}

/// The request `bardo-shell key-remap ACTION` sends.
pub fn key_remap_request(action: KeyRemapAction) -> Request {
    match action {
        KeyRemapAction::Status => Request::Status(StatusRequest::KeyRemap),
        KeyRemapAction::Retry => Request::KeyRemap(KeyRemapAsk::Retry),
        KeyRemapAction::Resume => Request::KeyRemap(KeyRemapAsk::Resume),
    }
}
