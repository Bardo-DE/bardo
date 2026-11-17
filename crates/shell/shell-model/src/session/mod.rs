//! The session's power actions: what logind offers, and what each is called (`words`).

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;
pub mod words;

pub use command::{SessionCommand, SessionEvent};
pub use model::{ActionReach, Offer, Session, SessionAction};
pub use step::step;

/// logind's power actions as a service.
///
/// Without logind the state stays at
/// `Session::default()` (absent: nothing offered), and the power menu shows every button
/// unavailable. Its `Deps` is `SessionLinks`: the locker, where `Run(Lock)` goes, and what Log Out,
/// Restart and Shut Down use to quit the apps first; the actions themselves go to `env`'s
/// session actions.
#[derive(Debug)]
pub enum SessionService {}

impl crate::service::Service for SessionService {
    const NAME: &'static str = "session";
    type State = Session;
    type Command = SessionCommand;
}
