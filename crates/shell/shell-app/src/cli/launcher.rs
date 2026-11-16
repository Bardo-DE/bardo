//! The launcher's command line: `bardo-shell launcher {toggle,open,close}` and `bardo-shell clipboard
//! {status,clear}`.

use clap::Subcommand;
use shell_ipc::request::Request;
use shell_ipc::show::LauncherRequest;
use shell_ipc::status::StatusRequest;
use shell_launcher::actions::ActionId;

use super::verbs::do_request;

/// `bardo-shell launcher ...`.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum LauncherAction {
    Toggle,
    Open {
        /// Text to start with.
        #[arg(long, default_value = "")]
        query: String,
    },
    Close,
}

/// `bardo-shell clipboard ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum ClipboardAction {
    /// Whether the clipboard history is kept, and how many entries it holds (never what).
    Status,
    /// Forget every entry now.
    Clear,
}

/// The request `bardo-shell launcher ACTION` sends.
pub fn launcher_request(action: &LauncherAction) -> Request {
    match action {
        LauncherAction::Toggle => do_request(ActionId::Launcher),
        LauncherAction::Open { query } => Request::Launcher(LauncherRequest::Open {
            query: query.clone(),
        }),
        LauncherAction::Close => Request::Launcher(LauncherRequest::Close),
    }
}

/// The request `bardo-shell clipboard ACTION` sends.
pub fn clipboard_request(action: ClipboardAction) -> Request {
    match action {
        ClipboardAction::Status => Request::Status(StatusRequest::Clipboard),
        ClipboardAction::Clear => Request::ClearClipboard,
    }
}
