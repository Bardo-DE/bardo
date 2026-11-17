//! The screen-sharing portal behind the Privacy page's list of apps that may share the screen
//! without asking again, taking one back, and letting another in.

mod bus;
#[cfg(feature = "testing")]
mod fake;
mod proxy;

pub use bus::OnBus;
#[cfg(feature = "testing")]
pub use fake::FakeScreenSharing;
