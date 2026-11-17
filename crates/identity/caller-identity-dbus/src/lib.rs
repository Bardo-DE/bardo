//! Which program is calling, over a bus: the adapter half of peephole.
//!
//! [`BusPeers`] turns a message's sender into a [`caller_identity_linux::Standing`]: it asks the bus for the
//! sender's process (the bus daemon's pidfd when it offers one), holds it, reads `/proc`
//! through [`caller_identity_linux::Prober`], asks the systemd manager about the unit when the table names
//! one, and judges. A method then calls [`BusPeers::require`] with the roles it accepts.
//!
//! Every call to the bus has a two-second bound ([`BUS_TIMEOUT`]) and fails closed. zbus runs on
//! its own executor by default (feature `async-io`); a tokio program enables `tokio` instead.

mod attest;
mod error;
mod manager;
mod offload;
mod owner;
mod who;

pub use caller_identity_core;
pub use caller_identity_linux;
pub use error::{UnitError, WhoError, access_denied};
pub use manager::{DROP_INS_MAX, ManagerLink, SYSTEMD, SYSTEMD_MANAGER, SYSTEMD_PATH, UnitSource};
pub use offload::{Cancelled, Offload};
pub use owner::OwnerWatch;
pub use who::{BusPeers, ManagerCheck};

/// How long any one call to the bus or the unit manager may take before the answer is "no": a
/// peer that does not answer must not wedge the one asking.
pub const BUS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
