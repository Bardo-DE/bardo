//! The battery: `UPower`'s display device, as the bar draws it.

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{BatteryCommand, BatteryEvent};
pub use model::{Battery, Charge, Remaining};
pub use step::step;

/// UPower's display device as a service. Without UPower the state stays at
/// `Battery::default()` (absent), and the bar hides the item.
#[derive(Debug)]
pub enum BatteryService {}

impl crate::service::Service for BatteryService {
    const NAME: &'static str = "battery";
    type State = Battery;
    type Command = BatteryCommand;
}
