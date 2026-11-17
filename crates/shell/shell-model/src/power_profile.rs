//! The power profile: power-profiles-daemon's words, as the control center draws them.

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{ProfileCommand, ProfileEvent};
pub use model::{PowerProfiles, Profile, ProfileDegraded};
pub use step::step;

/// power-profiles-daemon as a service. Without it the state stays at
/// `PowerProfiles::default()` (absent), and the control center hides the control.
#[derive(Debug)]
pub enum PowerProfileService {}

impl crate::service::Service for PowerProfileService {
    const NAME: &'static str = "power_profile";
    type State = PowerProfiles;
    type Command = ProfileCommand;
}
