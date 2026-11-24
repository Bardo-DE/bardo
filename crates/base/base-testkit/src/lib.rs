//! Test-only utilities that know nothing of the accounts service (a dev crate): a scratch directory
//! that removes itself, the one deadline tests wait with, and (feature `dbus`) a private session
//! bus. Every crate of this workspace may take it as a dev dependency without a cycle, because it
//! depends on nothing of ours. The fakes that do know the accounts service's types (providers,
//! accounts, sheets, the service) are in `account-fake`.

mod bus;
mod deadline;
mod eventually;
mod groups;
pub mod hang_guard;
mod reaped;
mod scratch;

#[cfg(feature = "dbus")]
pub use bus::name_of;
pub use bus::PrivateBus;
pub use deadline::{Deadline, GENEROUS};
pub use eventually::eventually_blocking;
#[cfg(feature = "tokio")]
pub use eventually::{eventually, eventually_some, eventually_some_async};
pub use reaped::Reaped;
pub use scratch::Scratch;
