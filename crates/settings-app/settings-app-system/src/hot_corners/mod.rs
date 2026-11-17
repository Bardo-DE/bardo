//! What the Hot Corners sheet asks of the computer: the actions installed apps offer, and a file
//! or a folder chosen by the person.

mod bus;
#[cfg(feature = "testing")]
mod fake;

pub use bus::OnCorners;
#[cfg(feature = "testing")]
pub use fake::FakeCorners;
