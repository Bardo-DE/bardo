//! sill's seat at intentd: the confirmation sheet it serves (`org.bardo.Confirm1`), the standing
//! terminal grants and the breaker it holds the `control` role over (`org.bardo.Intents1.Control`),
//! and the shell's own actions it provides as `org.bardo.Shell` (`IntentProvider1`). One
//! connection owns all three, because intentd names a caller's role from the well-known names its
//! connection owns.
//!
//! This is the pure half: what a confirmation queue is and how an answer, a withdrawal or an
//! expiry moves it ([`sheet`]), what the person sees of the terminal's grants ([`terminal`]), and
//! how an invocation of a shell action reads ([`shell`]). `shell-services` does the bus.

mod command;
mod model;
pub mod sheet;
pub mod shell;
mod step;
pub mod terminal;
#[cfg(test)]
mod tests;

pub use action_core::{ConfirmId, ConfirmOffer, Gesture, NeverOffer};
pub use action_vocab::ActionRef;
pub use command::{IntentsAsk, IntentsCommand, IntentsEffect, IntentsEvent};
pub use model::{Intents, IntentsLink};
pub use permission_core::StandingGrantId;
pub use provenance::Effect;
pub use step::step;
