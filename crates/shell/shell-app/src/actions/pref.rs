//! The preferences an action flips: the dock's auto-hide and magnification and Do Not Disturb. Each is written to
//! `settings.toml`, which stays the one truth; the surfaces follow once the file settles.

use shell_ipc::response::{FailureKind, Response};
use shell_model::actions::Pref;
use shell_settings::ShellFile;
use shell_ui_kit::links::Delivery;
use tokio::sync::watch;
use ui_settings::Store;

use crate::daemon::settings::write_key;
use crate::ipc::dispatch::{Reach, failed};

/// Flip `pref`.
pub fn flip(reach: &Reach, pref: Pref) -> Response {
    let (store, settings) = (reach.store, reach.settings);
    match pref {
        Pref::DockAutoHide => flip_key(store, settings, "dock.autohide", |file| file.dock.autohide),
        Pref::DockMagnification => flip_key(store, settings, "dock.magnification", |file| {
            file.dock.magnification
        }),
        Pref::Focus => toggle_focus(reach),
    }
}

/// Flip the `bool` key `key`, which `read` finds in the file, in `settings.toml`, and publish
/// the file at once so the surfaces follow before the watch has fired.
fn flip_key(
    store: &Store,
    settings: &watch::Sender<ShellFile>,
    key: &str,
    read: fn(&ShellFile) -> bool,
) -> Response {
    let now = !read(&crate::daemon::settings::load(store));
    if let Err(error) = write_key(store, key, &now) {
        return failed(
            FailureKind::Internal,
            format!("saving the settings: {error}"),
        );
    }
    settings.send_replace(crate::daemon::settings::load(store));
    Response::Done
}

/// Turn Do Not Disturb on when it is off, off when it is on.
fn toggle_focus(reach: &Reach) -> Response {
    let focus = &reach.links.focus;
    let next = !*focus.mode().borrow();
    match focus.set(next) {
        Delivery::Queued => Response::Done,
        Delivery::Gone => failed(
            FailureKind::Internal,
            "the focus setting has stopped".to_owned(),
        ),
    }
}
