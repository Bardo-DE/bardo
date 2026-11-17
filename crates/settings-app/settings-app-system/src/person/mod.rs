//! The person using this computer: their name and picture, for the card at the top of the sidebar.
//!
//! Read from AccountsService on the system bus, else from the passwd file and the
//! `.face` picture in their home folder. Nothing here starts a service.

mod bus;
#[cfg(feature = "testing")]
mod fake;

pub use bus::OnAccounts;
#[cfg(feature = "testing")]
pub use fake::FakePerson;
