//! The one transition.

use crate::bardo::{
    Active, Bardo, project_toplevels, reduce_app, reduce_note, reduce_space, reduce_window,
    set_active,
};
use crate::capture::reduce_captures;
use crate::event::DesktopEvent;
use crate::output::reduce_outputs;
use crate::state::DesktopState;
use crate::toplevel::{ToplevelEvent, reduce_toplevels};
use crate::workspace::reduce_workspaces;

/// `state` after `event`. Pure and total: an event for something unknown changes nothing.
#[must_use]
pub fn reduce(state: DesktopState, event: DesktopEvent) -> DesktopState {
    match event {
        DesktopEvent::Globals(protocols) => DesktopState { protocols, ..state },
        DesktopEvent::Toplevel(e) => toplevel(state, e),
        DesktopEvent::Workspace(e) => DesktopState {
            workspaces: reduce_workspaces(state.workspaces, e),
            ..state
        },
        DesktopEvent::Output(e) => DesktopState {
            outputs: reduce_outputs(state.outputs, e),
            ..state
        },
        DesktopEvent::Capture { id, event } => DesktopState {
            captures: reduce_captures(state.captures, id, event),
            ..state
        },
        DesktopEvent::App(e) => bardo(state, |q| reduce_app(q, e)),
        DesktopEvent::Window(e) => bardo(state, |q| reduce_window(q, e)),
        DesktopEvent::Space(e) => bardo(state, |q| reduce_space(q, e)),
        DesktopEvent::Active { app, key, actor } => {
            bardo(state, |q| set_active(q, Active { app, key, actor }))
        }
        DesktopEvent::Shell(e) => bardo(state, |q| reduce_note(q, e)),
    }
}

/// A toplevel event; a toplevel that closes takes its Bardo window with it.
fn toplevel(state: DesktopState, event: ToplevelEvent) -> DesktopState {
    let mut bardo = state.bardo;
    if let ToplevelEvent::Closed(key) = &event {
        bardo.forget_window(*key);
    }
    DesktopState {
        toplevels: reduce_toplevels(state.toplevels, event),
        bardo,
        ..state
    }
}

/// A Bardo event, applied by `step`; the windows' state is then projected onto the toplevels.
fn bardo(state: DesktopState, step: impl FnOnce(Bardo) -> Bardo) -> DesktopState {
    let bardo = step(state.bardo);
    DesktopState {
        toplevels: project_toplevels(state.toplevels, &bardo),
        bardo,
        ..state
    }
}
