//! The Show Desktop action, as macOS's: the first firing minimises the windows of the active
//! workspaces and remembers them; the next brings those back.

use shell_ipc::response::Response;
use shell_launcher::WindowKey;
use shell_model::ids::WorkspaceKey;
use shell_model::windows::{Toplevel, WlCommand};

use crate::ipc::dispatch::{Reach, sent};

/// The windows Show Desktop minimised, to bring back on its next firing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stash(pub Vec<WindowKey>);

/// Show Desktop, or bring the windows it put away back.
#[must_use]
pub fn toggle(reach: &Reach) -> Response {
    let wl = reach.services.windows.state.borrow().clone();
    let active: Vec<WorkspaceKey> = wl
        .workspaces
        .groups
        .iter()
        .filter_map(|group| group.active().map(|workspace| workspace.key))
        .collect();
    let mut stash = reach.stash.borrow_mut();
    let (commands, next) = plan(&wl.toplevels.windows, &active, &stash);
    *stash = next;
    let control = &reach.services.windows.control;
    sent(
        commands
            .into_iter()
            .try_for_each(|command| control.send(command)),
        "windows",
    )
}

/// Show Desktop: when `stash` holds a window still minimised, unminimise the stash's minimised
/// windows (in their order) and empty it; otherwise minimise every normal window on one of the
/// `active` workspaces and stash them.
pub fn plan(
    windows: &[Toplevel],
    active: &[WorkspaceKey],
    stash: &Stash,
) -> (Vec<WlCommand>, Stash) {
    let restore: Vec<WindowKey> = stash
        .0
        .iter()
        .copied()
        .filter(|key| windows.iter().any(|w| w.key == *key && w.minimized))
        .collect();
    if !restore.is_empty() {
        let commands = restore.into_iter().map(WlCommand::Unminimize).collect();
        return (commands, Stash::default());
    }
    let minimize: Vec<WindowKey> = windows
        .iter()
        .filter(|w| !w.minimized && w.workspaces.iter().any(|ws| active.contains(ws)))
        .map(|w| w.key)
        .collect();
    let commands = minimize.iter().copied().map(WlCommand::Minimize).collect();
    (commands, Stash(minimize))
}

#[cfg(test)]
mod tests {
    use shell_launcher::WindowKey;
    use shell_model::ids::WorkspaceKey;
    use shell_model::windows::Sizing;
    use shell_model::windows::{Toplevel, WlCommand};

    use super::{Stash, plan};

    fn window(key: u64, workspace: u64, minimized: bool) -> Toplevel {
        Toplevel {
            identifier: String::new(),
            key: WindowKey(key),
            app_id: format!("app{key}"),
            title: String::new(),
            active: false,
            minimized,
            sizing: Sizing::Floating,
            workspaces: vec![WorkspaceKey(workspace)],
            outputs: Vec::new(),
        }
    }

    #[test]
    fn show_desktop_minimises_the_workspace_then_brings_it_back() {
        let active = [WorkspaceKey(1)];
        let windows = vec![
            window(1, 1, false),
            window(2, 1, true),
            window(3, 2, false),
            window(4, 1, false),
        ];
        let (commands, stash) = plan(&windows, &active, &Stash::default());
        assert_eq!(
            commands,
            vec![
                WlCommand::Minimize(WindowKey(1)),
                WlCommand::Minimize(WindowKey(4))
            ]
        );
        assert_eq!(stash, Stash(vec![WindowKey(1), WindowKey(4)]));
        let hidden: Vec<_> = windows
            .into_iter()
            .map(|mut w| {
                if stash.0.contains(&w.key) {
                    w.minimized = true;
                }
                w
            })
            .collect();
        let (commands, after) = plan(&hidden, &active, &stash);
        assert_eq!(
            commands,
            vec![
                WlCommand::Unminimize(WindowKey(1)),
                WlCommand::Unminimize(WindowKey(4))
            ]
        );
        assert_eq!(after, Stash::default());
        let restored_by_hand = plan(&hidden[..1], &active, &Stash(vec![WindowKey(9)]));
        assert_eq!(
            restored_by_hand.1,
            Stash::default(),
            "a stash of gone windows starts over"
        );
    }
}
