//! Test-only utilities that know nothing of the accounts service (a dev crate): a scratch directory
//! that removes itself, the scratch HOME and XDG environment a started process gets ([`TestEnv`]),
//! the one deadline tests wait with, a child process that cannot outlive its test and (feature
//! `dbus`) a private session or system bus. Every crate of this workspace may take it as a dev
//! dependency without a cycle, because it depends on nothing of ours. The fakes that do know the
//! accounts service's types (providers, accounts, sheets, the service) are in `account-fake`.

mod bus;
mod deadline;
mod env;
mod eventually;
mod groups;
pub mod hang_guard;
mod reaped;
mod scratch;

pub use bus::PrivateBus;
#[cfg(feature = "dbus")]
pub use bus::name_of;
pub use deadline::{Deadline, GENEROUS};
pub use env::TestEnv;
pub use eventually::eventually_blocking;
#[cfg(feature = "tokio")]
pub use eventually::{eventually, eventually_some, eventually_some_async};
pub use reaped::Reaped;
pub use scratch::Scratch;
