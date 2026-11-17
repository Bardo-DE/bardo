//! The desktop-wide Spaces, over accountd's registry and memory's controls.
//!
//! That is the list, and the removal that first tells memory what becomes of the Space's memories
//! and history. Every call goes to the owner of the service's name that was confirmed, never to the name.

mod bus;
#[cfg(feature = "testing")]
mod fake;

pub use bus::OnBus;
#[cfg(feature = "testing")]
pub use fake::FakeSpaces;
