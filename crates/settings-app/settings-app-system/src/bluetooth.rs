//! BlueZ behind the Bluetooth page: one handle the environment carries, so a test hands in a
//! service that only answers from memory.

mod actions;
mod agent;
mod bus;
#[cfg(feature = "testing")]
mod fake;
mod read;
mod session;

pub use bus::OnBluetooth;
#[cfg(feature = "testing")]
pub use fake::{BtAsked, FakeBluetooth};
