//! `org.bardo.Compositor1` on the session bus: the compositor's door for the agent tier.

use compositor_dbus::{BUS_NAME, Compositor1Proxy};
use std::os::fd::OwnedFd;

// The calls cuad makes are the compositor's `Compositor1`. The role is the compositor's to check: a
// connection that is not allowed the agent role gets an access-denied error, which surfaces as
// the call failing.

/// The agent role's name in `Connect`.
const AGENT: &str = "agent";

/// Whether the compositor is on the bus, and how to reach it.
#[derive(Debug, Clone)]
pub struct Door {
    proxy: Compositor1Proxy<'static>,
}

impl Door {
    /// The door when `org.bardo.Compositor1` has an owner on `connection`; `None` when no
    /// compositor of ours is running (or the bus cannot say).
    pub async fn find(connection: &zbus::Connection) -> Option<Door> {
        let bus = zbus::fdo::DBusProxy::new(connection).await.ok()?;
        let name = zbus::names::BusName::try_from(BUS_NAME).ok()?;
        if !bus.name_has_owner(name).await.ok()? {
            return None;
        }
        let proxy = Compositor1Proxy::new(connection).await.ok()?;
        Some(Door { proxy })
    }

    /// A new Wayland connection with the agent role.
    ///
    /// # Errors
    ///
    /// The D-Bus error of the call that asks for the connection.
    pub async fn agent_connection(&self) -> Result<OwnedFd, zbus::Error> {
        Ok(self.proxy.connect(AGENT).await?.into())
    }

    pub(super) fn proxy(&self) -> &Compositor1Proxy<'static> {
        &self.proxy
    }
}
