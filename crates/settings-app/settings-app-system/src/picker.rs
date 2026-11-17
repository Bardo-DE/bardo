//! Setting a row of inferd's settings module for the model picker: the module answers when it has
//! put the row in force.

mod bus;
#[cfg(feature = "testing")]
mod fake;
mod set;

pub use bus::OnClient;
#[cfg(feature = "testing")]
pub use fake::{FakePicker, PickerAsked};
