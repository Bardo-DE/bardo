//! The computers over inferd, through inferd's client: the places, the Tailscale candidates, the
//! guests and the changes. The engines signal (a computer answering or not) comes from its proxy.

mod bus;
#[cfg(feature = "testing")]
mod fake;

pub use bus::{OnBus, relay};
#[cfg(feature = "testing")]
pub use fake::FakeComputers;
