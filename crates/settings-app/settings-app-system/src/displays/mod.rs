//! The compositor behind the Displays page: one handle the environment carries, so a test hands
//! in a compositor that only answers from memory.

mod brightness;
mod bus;
#[cfg(feature = "testing")]
mod fake;
mod map;

pub use bus::OnCompositor;
#[cfg(feature = "testing")]
pub use fake::FakeDisplays;
