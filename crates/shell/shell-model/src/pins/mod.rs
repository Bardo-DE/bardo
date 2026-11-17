//! The dock's pinned apps, in order.

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{PinAt, PinEvent};
pub use model::DockPins;
pub use step::step;

/// The dock pins as a service.
///
/// Loaded at once, then every [`PinEvent`] sent to the control applied and the file written atomically after each; an external edit (another process, a
/// text editor) is picked up through a 30 ms debounced directory watch and published as if it
/// were a `Replaced` event. A fresh account's dock does not exist: once the installed apps are
/// known the dock starts with the desktop's own (`shell_model::seed::first_dock`), once.
#[derive(Debug)]
pub enum PinsService {}

impl crate::service::Service for PinsService {
    const NAME: &'static str = "pins";
    type State = DockPins;
    type Command = PinEvent;
}
