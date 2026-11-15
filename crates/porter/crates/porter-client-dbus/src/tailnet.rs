//! The person's own computers on their Tailscale network (`org.quire.Tailnet1` on accountd), for
//! the shell, Settings and the terminal. Nothing here reads Tailscale: accountd does, once, for
//! the Tailscale account the person added.
//!
//! ```ignore
//! let tailnet = bus.tailnet();
//! let mut changes = tailnet.watch().await?;          // subscribe before reading the list
//! for machine in tailnet.machines().await? {          // Machine { node, name, dns, owner, .. }
//!     /* connect to machine.dns; key pins and consent by machine.node */
//! }
//! while changes.next().await.is_some() { /* read the list again */ }
//! ```

use crate::Bus;
use crate::failure::link_error;
use futures_util::StreamExt;
use porter_client::{Changes, LinkError, Malformed};
use porter_dbus::accounts::{TailnetProxy, machine_from_dbus};
use porter_tailscale::Machine;

/// accountd's list of the person's computers, as an app uses it over the bus.
#[derive(Debug, Clone)]
pub struct Tailnet {
    bus: Bus,
}

impl Bus {
    /// The handle on tailnet.
    pub fn tailnet(&self) -> Tailnet {
        Tailnet { bus: self.clone() }
    }
}

impl Bus {
    async fn tailnet_proxy(&self) -> Result<TailnetProxy<'static>, LinkError> {
        self.accounts
            .proxy(&self.connection)
            .await
            .map_err(|e| link_error(&e))
    }
}

impl Tailnet {
    /// The other computers on the person's network, none for this one, in accountd's order. The
    /// list is empty with no Tailscale account, or one that is not working: the account's own
    /// state says why.
    pub async fn machines(&self) -> Result<Vec<Machine>, LinkError> {
        self.bus
            .tailnet_proxy()
            .await?
            .machines()
            .await
            .map_err(|e| link_error(&e))?
            .iter()
            .map(|row| machine_from_dbus(row).map_err(|_| LinkError::Malformed(Malformed::Row)))
            .collect()
    }

    /// The changes to the list, from now on: one item each time the list changed, with nothing in
    /// it; read the list again. It subscribes, then reads the list once so accountd counts this
    /// connection among those it tells.
    pub async fn watch(&self) -> Result<Changes<()>, LinkError> {
        let proxy = self.bus.tailnet_proxy().await?;
        let stream = proxy.receive_changed().await.map_err(|e| link_error(&e))?;
        // Reading the list once is how accountd counts this connection among those it tells.
        proxy.machines().await.map_err(|e| link_error(&e))?;
        Ok(Changes::of(stream.map(|_| ())))
    }
}
