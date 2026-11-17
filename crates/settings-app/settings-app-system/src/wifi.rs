//! The network service behind the Wi-Fi page: one handle the environment carries, so a test hands
//! in a service that only answers from memory.

mod bus;
#[cfg(feature = "testing")]
mod fake;
mod join;
mod known;
mod profile;
mod read;

pub use bus::OnWifi;
#[cfg(feature = "testing")]
pub use fake::{FakeWifi, WifiAsked};
