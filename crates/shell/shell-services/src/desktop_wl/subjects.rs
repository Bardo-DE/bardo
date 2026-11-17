//! The program behind each window's app, as the compositor established it.
//!
//! This is `bardo_desktop_app_v1.subject`, projected for the windows' published state. Only the
//! Bardo arm has any: the standard protocols say nothing of the program behind a window.

use desktop_client::DesktopState;
use shell_model::windows::Subjects;

/// The subject of each app id on screen, from Bardo windows joined to their toplevels.
///
/// An app id whose windows report different subjects, or only some of them one, has
/// none (`Subjects::collect`).
#[must_use]
pub fn project(state: &DesktopState) -> Subjects {
    Subjects::collect(state.bardo.windows().filter_map(|window| {
        let toplevel = state.toplevels.get(window.key)?;
        let subject = window
            .app
            .and_then(|key| state.bardo.app(key))
            .and_then(|app| app.subject.as_ref())
            .map(|subject| subject.as_str().to_owned());
        Some((toplevel.app_id.clone(), subject))
    }))
}
