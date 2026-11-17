//! Following a daemon's live settings module (`org.bardo.SettingsModule1`) as a pane does, over
//! the bus.

mod bus;
#[cfg(feature = "testing")]
mod caller;
#[cfg(feature = "testing")]
mod fake;

pub use bus::OnBus;
#[cfg(feature = "testing")]
pub use fake::FakeModules;
