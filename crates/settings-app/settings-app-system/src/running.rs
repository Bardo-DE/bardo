//! The models inferd runs itself: reading them, starting and stopping one, and the signal that
//! says to read again. Looking never starts the assistant.

mod bus;
#[cfg(feature = "testing")]
mod fake;

pub use bus::OnBus;
#[cfg(feature = "testing")]
pub use fake::FakeRunning;
