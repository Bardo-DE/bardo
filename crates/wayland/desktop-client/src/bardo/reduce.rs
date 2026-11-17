//! Bardo transition.

use super::app::AppEvent;
use super::lifecycle::reduce_entity;
use super::set::{Active, Bardo};
use super::shell::{ShellEvent, reduce_shell};
use super::space::SpaceEvent;
use super::window::{SpacePlace, WindowEvent};

/// `bardo` after an app event. Pure and total: a change to something unknown is ignored, and a
/// `Done` marks its object announced.
pub(crate) fn reduce_app(mut bardo: Bardo, event: AppEvent) -> Bardo {
    if let Some(key) = reduce_entity(&mut bardo.apps, event) {
        if bardo.active.app == Some(key) {
            bardo.active.app = None;
        }
        for w in bardo.windows.values_mut() {
            if w.app == Some(key) {
                w.app = None;
            }
        }
    }
    bardo
}

/// `bardo` after a window event (a window is created by its `Opened`).
pub(crate) fn reduce_window(mut bardo: Bardo, event: WindowEvent) -> Bardo {
    if let Some(key) = reduce_entity(&mut bardo.windows, event) {
        bardo.forget_window(key);
    }
    bardo
}

/// `bardo` after a Space event.
pub(crate) fn reduce_space(mut bardo: Bardo, event: SpaceEvent) -> Bardo {
    if let Some(key) = reduce_entity(&mut bardo.spaces, event) {
        for w in bardo.windows.values_mut() {
            if w.place == SpacePlace::On(key) {
                w.place = SpacePlace::Unknown;
            }
        }
    }
    bardo
}

/// `bardo` with its active app and key window replaced.
pub(crate) fn set_active(mut bardo: Bardo, active: Active) -> Bardo {
    bardo.active = active;
    bardo
}

/// `bardo` after a note from the shell.
pub(crate) fn reduce_note(mut bardo: Bardo, event: ShellEvent) -> Bardo {
    bardo.shell = reduce_shell(bardo.shell, event);
    bardo
}
