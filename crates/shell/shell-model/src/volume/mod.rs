//! The default sink's volume and mute, as the bar and the OSD draw them.

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{Step, VolumeCommand, VolumeEvent};
pub use model::{Mute, Volume};
pub use step::{step, stepped};

/// The output volume as a service.
///
/// It is libpulse's standard mainloop on a thread of the blocking
/// pool, asleep until the sound server or a command wakes it, its callbacks feeding `step`.
/// Without a pulse server the state stays at `Volume::default()` (absent).
#[derive(Debug)]
pub enum VolumeService {}

impl crate::service::Service for VolumeService {
    const NAME: &'static str = "volume";
    type State = Volume;
    type Command = VolumeCommand;
}
