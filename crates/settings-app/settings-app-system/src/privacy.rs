//! The calls the Privacy page makes on the compositor's module: allowing or denying a power for a
//! program, and asking again.

mod bus;
#[cfg(feature = "testing")]
mod fake;
mod set;

pub use bus::OnClient;
#[cfg(feature = "testing")]
pub use fake::{FakePrivacy, PrivacyAsked};
