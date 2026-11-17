//! The desktop's menu permit: only the shell may press an item.

use super::permit::{Caller, MenuPermit, Verdict};
use caller_identity_dbus::BusPeers;
use caller_identity_dbus::caller_identity_core::{Proof, Requirement, roles};
use std::sync::Arc;

/// Lets the shell press items and no one else: the caller must stand as the `shell` role of the
/// desktop's caller table at the full proof (its image is root-owned and its unit is the real
/// one), as the caller-identity library judges it. Any caller the caller-identity library cannot identify is refused.
///
/// The `BusPeers` is the consumer's own, built once for the process on a connection to the same
/// bus (not the menu's own, which does not exist before the export starts).
#[derive(Debug, Clone)]
pub struct ShellOnly {
    peers: Arc<BusPeers>,
}

impl ShellOnly {
    /// A permit that asks `peers` who the caller is.
    pub fn new(peers: Arc<BusPeers>) -> ShellOnly {
        ShellOnly { peers }
    }
}

impl MenuPermit for ShellOnly {
    async fn permit(&self, caller: &Caller) -> Verdict {
        let need = [Requirement::new(roles::SHELL, Proof::Full)];
        match self
            .peers
            .require(Some(caller.sender.0.as_str()), &need)
            .await
        {
            Ok(_) => Verdict::Allow,
            Err(_) => Verdict::Refuse,
        }
    }
}
