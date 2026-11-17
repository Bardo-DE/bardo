//! The sync service's side of the Accounts pane: what it says of a dataset, and the deletions it
//! holds for the person's word. Looked for, never started.

mod bus;
#[cfg(feature = "testing")]
mod fake;
mod holds;
mod status;

pub use bus::OnBus;
#[cfg(feature = "testing")]
pub use fake::{FakeSync, SyncAsked};
