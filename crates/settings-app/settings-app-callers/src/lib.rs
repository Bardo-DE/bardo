//! Which program is on the other end of a bus name or a bus message.
//!
//! A well-known name on the session bus goes to whoever asks first, so Settings never trusts one:
//! before it reads a service, or hands it a typed secret, it asks whether the owner of the name is
//! the program that plays the service's role in the desktop's caller table ([`RoleCheck`], the
//! `OwnerCheck` the UI kit's live client asks). The same peers judge who may ask the running Settings to
//! show itself (`settings-app-intents`). Everything here is the caller-identity library's: this crate names the roles and
//! the proof each question needs, and decides nothing about a process itself.

mod allow;
mod check;
#[cfg(feature = "dev-callers")]
mod dev;
mod peers;
mod services;
#[cfg(feature = "testing")]
mod testing;

pub use allow::allowing;
pub use caller_identity_dbus::caller_identity_core::{
    Proof, Refused, Requirement, RoleName, roles,
};
pub use caller_identity_dbus::{BusPeers, access_denied};
pub use check::RoleCheck;
#[cfg(feature = "dev-callers")]
pub use dev::{DevRowsFault, load_dev_rows, use_dev_rows};
pub use peers::desktop_peers;
pub use services::{SERVICE_PROOF, required_role};
#[cfg(feature = "testing")]
pub use testing::{introduce_service, private_peers};
