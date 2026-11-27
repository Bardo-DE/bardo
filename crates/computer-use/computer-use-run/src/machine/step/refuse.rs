//! Refusing an input action before it is asked of the router or injected: the run's own checks
//! on what the model sent, from what the last look showed.

use crate::machine::ops::nodes_of;
use crate::machine::run::Run;
use crate::observation::Observation;
use crate::shortcut_guard::control_for;
use computer_use_action::{ActionClass, ComputerUseAction};
use computer_use_bus::LocalRefusal;
use computer_use_geometry::WindowSpace;

/// Why the action must not run, if it must not: it repeats the last input on a screen that did
/// not change, or it is a chord for a control the window shows.
pub(super) fn before_input(
    run: &Run,
    obs: &Observation,
    action: &ComputerUseAction<WindowSpace>,
) -> Option<LocalRefusal> {
    if !matches!(action.class(), ActionClass::Pointer | ActionClass::Keyboard) {
        return None;
    }
    let repeats = run.track.last_input.as_ref() == Some(action);
    if repeats && run.track.unchanged {
        return Some(LocalRefusal::NoProgress);
    }
    control_for(action, nodes_of(obs)).map(|_| LocalRefusal::HasControl)
}
