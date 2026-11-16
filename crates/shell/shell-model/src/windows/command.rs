//! What the `windows` thread can be told to do.

use serde::{Deserialize, Serialize};
use shell_launcher::WindowKey;

use crate::ids::WorkspaceKey;

/// A request to the compositor, carried out on the thread's own connection.
///
/// There is no minimize-rectangle command (`SetMinimizeRect`): `set_rectangle` needs the dock's
/// `wl_surface`, which lives on layer-shell-host's connection, not this one, so it is left out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum WlCommand {
    /// Activate (and unminimize) a toplevel on the thread's seat.
    Activate(WindowKey),
    /// Ask a toplevel to close.
    Close(WindowKey),
    Minimize(WindowKey),
    Unminimize(WindowKey),
    /// Make a workspace the active one of its group.
    ActivateWorkspace(WorkspaceKey),
    /// Reorder: put `workspace` before or after `other` (bar drag reorder).
    MoveWorkspace {
        workspace: WorkspaceKey,
        relative: Relative,
        other: WorkspaceKey,
    },
    /// Pin a workspace (needs [`crate::windows::WorkspaceCap::Pin`]).
    PinWorkspace(WorkspaceKey),
    /// Unpin a workspace (needs [`crate::windows::WorkspaceCap::Pin`]).
    UnpinWorkspace(WorkspaceKey),
    /// Move a toplevel onto a workspace, on that workspace's output (needs the toplevel
    /// manager's `move_to_ext_workspace`, version 4).
    MoveToplevelToWorkspace {
        toplevel: WindowKey,
        workspace: WorkspaceKey,
    },
}

/// Which side of another workspace a moved one lands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relative {
    Before,
    After,
}
