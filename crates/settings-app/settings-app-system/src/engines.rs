//! llama.cpp on this computer, as the assistant lists it, and the assistant's restart.
//!
//! The restart is in the person's own session. Looking never starts the assistant: with it not running there is nothing
//! to say, and opening Settings must not start it. Restarting is the person's word.

mod bus;
#[cfg(feature = "testing")]
mod fake;

pub use bus::{OnBus, restart};
#[cfg(feature = "testing")]
pub use fake::FakeEngines;
