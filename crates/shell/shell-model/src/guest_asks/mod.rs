//! Another of the person's computers asking to borrow this computer's models, as the shell tells
//! the person.
//!
//! A notification per asking computer, "Let <name> use this computer's models?", with Allow and
//! Deny. A press answers inferd; the notification is withdrawn when the question is answered, here
//! or elsewhere (Settings).

mod command;
mod model;
mod notice;
mod step;
#[cfg(test)]
mod tests;

pub use command::{GuestAsksCommand, GuestAsksEffect, GuestAsksEvent};
pub use model::{Ask, Computer, GuestAsksState, Verdict};
pub use notice::{NOTICE_ENTRY, Notice, choice_key, choice_of, notice_text, shown_for};
pub use step::step;

/// The questions about lending models on `env`'s session bus, told through the notifications
/// service in its `GuestAsksLinks`.
#[derive(Debug)]
pub enum GuestAsksService {}

impl crate::service::Service for GuestAsksService {
    const NAME: &'static str = "guest_asks";
    type State = GuestAsksState;
    type Command = GuestAsksCommand;
}
