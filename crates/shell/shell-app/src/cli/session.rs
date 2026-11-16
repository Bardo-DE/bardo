//! The session's command line: `bardo-shell lock [now|status]` and its pure mapping to a request.

use clap::ValueEnum;
use shell_ipc::request::Request;
use shell_ipc::status::StatusRequest;
use shell_launcher::actions::ActionId;

/// `bardo-shell lock [VERB]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LockVerb {
    /// Lock now.
    Now,
    /// What the lock screen is doing.
    Status,
}

/// The request `bardo-shell lock VERB` sends.
pub fn lock_request(verb: LockVerb) -> Request {
    match verb {
        LockVerb::Now => super::verbs::do_request(ActionId::Lock),
        LockVerb::Status => Request::Status(StatusRequest::Lock),
    }
}
