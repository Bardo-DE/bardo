//! The keyboard backlight: `UPower`'s `KbdBacklight` level, as the control center draws it.

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{KbdCommand, KbdEvent};
pub use model::{KbdBacklight, KbdLevel};
pub use step::{step, step_for};

/// The keyboard backlight as a service. Without UPower or a backlight the state stays at
/// `KbdBacklight::default()` (absent), and the control center shows no module.
#[derive(Debug)]
pub enum KbdBacklightService {}

impl crate::service::Service for KbdBacklightService {
    const NAME: &'static str = "kbd_backlight";
    type State = KbdBacklight;
    type Command = KbdCommand;
}
