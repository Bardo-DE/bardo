//! The account service as the Accounts pane meets it.
//!
//! That is the accounts the live modules list, and the sheet that adds an account (an identity or
//! an AI key, through accountd), asked of the owner of the service's name that was confirmed.

mod add;
mod bus;
#[cfg(feature = "testing")]
mod fake;
mod read;

pub use bus::OnBus;
#[cfg(feature = "testing")]
pub use fake::{FakeAccounts, SheetAsked};
pub use read::read_accounts;
