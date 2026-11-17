//! Wi-Fi: `NetworkManager`'s wireless radio and the networks in range, as the control center lists
//! them.

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{WifiCommand, WifiEvent};
pub use model::{AccessPoint, Association, Radio, Saved, Security, Ssid, Wifi, listed_networks};
pub use step::step;

/// NetworkManager's wireless radio as a service. Without NetworkManager or a wireless device
/// the state stays at `Wifi::default()` (absent), and the control center hides the module's list.
#[derive(Debug)]
pub enum WifiService {}

impl crate::service::Service for WifiService {
    const NAME: &'static str = "wifi";
    type State = Wifi;
    type Command = WifiCommand;
}
