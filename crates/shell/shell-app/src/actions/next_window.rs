//! The `next-window` action: the next window of the front app (Command+` on a Mac). The toplevel list
//! is most recently activated first, so the window to bring forward is the front app's least
//! recently used one on the front window's workspaces; with one window there is nothing to do.

use shell_ipc::response::Response;
use shell_model::windows::{Toplevel, Toplevels, WlCommand};

use crate::ipc::dispatch::{Reach, sent};

/// Bring the front app's next window forward.
#[must_use]
pub fn ask(reach: &Reach) -> Response {
    let handle = &reach.services.windows;
    let command = next(&handle.state.borrow().toplevels);
    match command {
        Some(command) => sent(handle.control.send(command), "windows"),
        None => Response::Done,
    }
}

/// The command that brings the next window of the front app forward, if it has another.
fn next(all: &Toplevels) -> Option<WlCommand> {
    let front = all.active()?;
    let mine: Vec<&Toplevel> = all
        .windows
        .iter()
        .filter(|w| w.app_id == front.app_id)
        .filter(|w| w.workspaces.iter().any(|ws| front.workspaces.contains(ws)))
        .collect();
    match mine.as_slice() {
        [_, .., last] => Some(WlCommand::Activate(last.key)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use shell_launcher::WindowKey;
    use shell_model::ids::WorkspaceKey;
    use shell_model::windows::{Sizing, Toplevel, Toplevels, WlCommand};

    use super::next;

    fn window(key: u64, app: &str, active: bool, workspace: u64) -> Toplevel {
        Toplevel {
            identifier: String::new(),
            key: WindowKey(key),
            app_id: app.to_owned(),
            title: String::new(),
            active,
            minimized: false,
            sizing: Sizing::Floating,
            workspaces: vec![WorkspaceKey(workspace)],
            outputs: Vec::new(),
        }
    }

    fn list(windows: Vec<Toplevel>) -> Toplevels {
        Toplevels { windows }
    }

    #[test]
    fn the_least_recent_window_of_the_front_app_comes_forward() {
        let all = list(vec![
            window(1, "foot", true, 1),
            window(2, "firefox", false, 1),
            window(3, "foot", false, 1),
            window(4, "foot", false, 2),
        ]);
        assert_eq!(next(&all), Some(WlCommand::Activate(WindowKey(3))));
    }

    #[test]
    fn one_window_or_none_active_does_nothing() {
        let one = list(vec![window(1, "foot", true, 1)]);
        assert_eq!(next(&one), None);
        let none = list(vec![window(1, "foot", false, 1)]);
        assert_eq!(next(&none), None);
    }
}
