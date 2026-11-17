//! The network: `NetworkManager`'s primary connection and connectivity, as the bar draws them.

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{NetworkCommand, NetworkEvent};
pub use model::{Activating, Connectivity, Link, Network};
pub use step::step;

/// NetworkManager's primary connection as a service. Without NetworkManager the state
/// stays at `Network::default()` (disconnected, connectivity unknown).
#[derive(Debug)]
pub enum NetworkService {}

impl crate::service::Service for NetworkService {
    const NAME: &'static str = "network";
    type State = Network;
    type Command = NetworkCommand;
}
