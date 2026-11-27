//! This computer, as its own Tailscale says it is.

use std::net::IpAddr;
use tailscale_types::NodeId;
use tailscale_types::{Standing, Status, UserId};

/// Who this computer is on the network: the id Tailscale keeps for it, the user it belongs to,
/// the name people call it by and the addresses the network gave it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    node: NodeId,
    user: UserId,
    name: String,
    addresses: Vec<IpAddr>,
}

impl Identity {
    /// An identity of these parts.
    #[must_use]
    pub fn new(node: NodeId, user: UserId, name: String, addresses: Vec<IpAddr>) -> Self {
        Self {
            node,
            user,
            name,
            addresses,
        }
    }

    /// This computer as `status` says, when Tailscale is running and signed in; none otherwise.
    #[must_use]
    pub fn of(status: &Status) -> Option<Self> {
        if status.standing() != Standing::Ready {
            return None;
        }
        let me = status.me.as_ref()?;
        Some(Self::new(
            me.id.clone(),
            me.user,
            me.name().to_owned(),
            me.addresses.clone(),
        ))
    }

    /// Its stable id.
    #[must_use]
    pub fn node(&self) -> &NodeId {
        &self.node
    }

    /// The user it belongs to.
    #[must_use]
    pub fn user(&self) -> UserId {
        self.user
    }

    /// The name people call it by.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The addresses the network gave it.
    #[must_use]
    pub fn addresses(&self) -> &[IpAddr] {
        &self.addresses
    }

    /// Whether `ip` is one of its own addresses.
    #[must_use]
    pub fn owns(&self, ip: IpAddr) -> bool {
        self.addresses.contains(&ip)
    }
}
