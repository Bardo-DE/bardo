//! Bardo state seen as toplevel state.

use super::set::Bardo;
use super::window::BardoWindow;
use crate::toplevel::{ToplevelFlag, ToplevelFlags, Toplevels};
use desktop_protocols::vocab::WindowStateKind;

/// `toplevels` with the flags, parent and output of every toplevel that has a Bardo window
/// taken from that window, so a consumer that reads only toplevels sees Bardo arm's state
/// as it sees wlr's: filled is maximised, and the key window is activated.
pub(crate) fn project_toplevels(mut toplevels: Toplevels, bardo: &Bardo) -> Toplevels {
    for (key, window) in &bardo.windows {
        if let Some(toplevel) = toplevels.0.get_mut(key) {
            toplevel.flags = flags_of(window, bardo.active.key == Some(*key));
            toplevel.parent = window.parent;
            if let Some(output) = &window.output {
                toplevel.outputs = vec![output.clone()];
            }
        }
    }
    toplevels
}

/// The wlr-style flags of a window; `key` says it is the key window.
fn flags_of(window: &BardoWindow, key: bool) -> ToplevelFlags {
    let from_state = match window.state {
        WindowStateKind::Floating | WindowStateKind::Tiled => None,
        WindowStateKind::Filled => Some(ToplevelFlag::Maximized),
        WindowStateKind::FullScreen => Some(ToplevelFlag::Fullscreen),
        WindowStateKind::Minimised => Some(ToplevelFlag::Minimized),
    };
    let activated = key.then_some(ToplevelFlag::Activated);
    ToplevelFlags::of(from_state.into_iter().chain(activated))
}
