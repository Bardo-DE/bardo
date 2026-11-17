//! The update service behind the Software Update page: one handle the environment carries, so a
//! test hands in a service that only answers from memory.

mod bus;
#[cfg(feature = "testing")]
mod fake;
mod leave;
mod wire;

pub use bus::OnBus;
#[cfg(feature = "testing")]
pub use fake::{FakeLeaves, FakeUpdates, NOW, UpdatesAsked};
pub use leave::PowerMenu;
