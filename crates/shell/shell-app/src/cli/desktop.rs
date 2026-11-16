//! The desktop's command line: `bardo-shell dock`. The verbs
//! that are a wire enum's words (`bardo-shell power-menu`, `bardo-shell force-quit`, `bardo-shell screenshot`,
//! `bardo-shell companion`, `bardo-shell voice`) take the enum itself, parsed by `word::word`.

use clap::Subcommand;
use shell_ipc::request::Request;
use shell_launcher::AppId;
use shell_launcher::actions::ActionId;
use shell_model::actions::{Ask, Subject};

use super::verbs::do_request;

/// `bardo-shell dock ...`.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum DockAction {
    /// Keep APP (a desktop id, `org.mozilla.firefox`) in the dock.
    Pin { app: String },
    /// Stop keeping APP; a running app keeps its tile.
    Unpin { app: String },
    /// Hide the dock until the pointer reaches its edge when it is shown, keep it shown when it is
    /// hidden (flips `dock.autohide`).
    ToggleHide,
}

/// The request `bardo-shell dock ACTION` sends.
pub fn dock_request(action: &DockAction) -> Request {
    match action {
        DockAction::Pin { app } => app_request(ActionId::PinApp, app),
        DockAction::Unpin { app } => app_request(ActionId::UnpinApp, app),
        DockAction::ToggleHide => do_request(ActionId::ToggleDockHide),
    }
}

/// The request that does `action` to the app with desktop id `app`.
fn app_request(action: ActionId, app: &str) -> Request {
    Request::Do(Ask::on(action, Subject::App(AppId(app.to_owned()))))
}
