//! The identities signed in on this machine (`org.bardo.Identities1` on accountd), for an app
//! that wants to say "Continue as ada@example.org" without holding the sign-in itself. An
//! identity is an address, a provider and a kind; nothing here is a token.
//!
//! ```ignore
//! let identities = bus.identities();
//! match identities.offer(&ProviderHint::Any, &window).await {   // the person picks in a sheet
//!     Ok(picked) => {
//!         // Sign in to your own service as `picked.address`, then say so:
//!         identities.register_use(&picked.id).await?;
//!         // A password identity may hand its password over, once, right after the pick:
//!         let password = identities.take_password(&picked.id).await?;
//!     }
//!     Err(IdentitiesError::Cancelled) => { /* the person closed the sheet */ }
//!     Err(IdentitiesError::NoIdentity) => { /* offer to sign one in */ }
//!     Err(other) => report(other),
//! }
//! let mut removed = identities.watch_removals().await?;        // subscribe before reading the list
//! while let Some(id) = removed.next().await { /* forget your own copies of it */ }
//! ```

use crate::Bus;
use crate::failure::{identities_error, link_error};
use account_client::{Changes, IdentitiesError, LinkError, Malformed, Picked};
use account_core::SecretText;
use account_core::{AppName, Identity, IdentityAddress, IdentityId, ProviderHint, ProviderId};
use account_dbus::accounts::{IdentitiesProxy, IdentityRemoved, identity_from_dbus};
use account_wire::ParentWindow;
use futures_util::StreamExt;
use std::io::Read;

/// accountd's identities, as an app uses it over the bus.
#[derive(Debug, Clone)]
pub struct Identities {
    bus: Bus,
}

impl Bus {
    /// The handle on identities.
    pub fn identities(&self) -> Identities {
        Identities { bus: self.clone() }
    }
}

impl Bus {
    async fn identities_proxy(&self) -> Result<IdentitiesProxy<'static>, IdentitiesError> {
        self.accounts
            .proxy(&self.connection)
            .await
            .map_err(|e| IdentitiesError::Link(link_error(&e)))
    }
}

fn malformed(what: Malformed) -> IdentitiesError {
    IdentitiesError::Link(LinkError::Malformed(what))
}

fn removed(signal: IdentityRemoved) -> Result<IdentityId, Malformed> {
    let args = signal.args().map_err(|_| Malformed::Signal)?;
    IdentityId::parse(args.identity_id).map_err(|_| Malformed::Id)
}

impl Identities {
    /// The identities signed in on this machine, in accountd's order. Local runtimes, Tailscale,
    /// AI keys and agent logins are not among them. A row that cannot be read fails the list.
    pub async fn list(&self) -> Result<Vec<Identity>, IdentitiesError> {
        let proxy = self.bus.identities_proxy().await?;
        proxy
            .list()
            .await
            .map_err(|e| identities_error(&e))?
            .iter()
            .map(|row| identity_from_dbus(row).map_err(|_| malformed(Malformed::Row)))
            .collect()
    }

    /// "Continue as": shows the person the identity picker and waits for the pick. `hint` limits
    /// it to one provider's identities, or [`ProviderHint::Any`]; the sheet sits over `window`.
    /// The app has one picker open at a time ([`IdentitiesError::Busy`]); the person closing it is
    /// [`IdentitiesError::Cancelled`].
    pub async fn offer(
        &self,
        hint: &ProviderHint,
        window: &ParentWindow,
    ) -> Result<Picked, IdentitiesError> {
        let proxy = self.bus.identities_proxy().await?;
        let word = match hint {
            ProviderHint::Provider(provider) => provider.as_str(),
            _ => "",
        };
        let parent = match window {
            ParentWindow::Handle(handle) => handle.as_str(),
            _ => "",
        };
        let (id, address, provider) = proxy
            .offer(word, parent)
            .await
            .map_err(|e| identities_error(&e))?;
        Ok(Picked::new(
            IdentityId::parse(&id).map_err(|_| malformed(Malformed::Id))?,
            IdentityAddress(address),
            ProviderId::parse(&provider).map_err(|_| malformed(Malformed::Id))?,
        ))
    }

    /// The password (or app password) of the identity the person just picked in this app's
    /// [`Identities::offer`]. It is handed over once per pick, only for a password identity, and
    /// accountd records that it was; any other call is `Link(LinkError::Denied)`. Keep it in the
    /// app's own secret store.
    pub async fn take_password(&self, id: &IdentityId) -> Result<SecretText, IdentitiesError> {
        let proxy = self.bus.identities_proxy().await?;
        let fd = proxy
            .take_password(id.as_str())
            .await
            .map_err(|e| identities_error(&e))?;
        let mut text = String::new();
        std::fs::File::from(std::os::fd::OwnedFd::from(fd))
            .read_to_string(&mut text)
            .map_err(|_| malformed(Malformed::Descriptor))?;
        Ok(SecretText::new(text))
    }

    /// Says that this app signed in with `id`: it joins the list Settings shows as the apps using
    /// the identity. Saying it again changes nothing.
    pub async fn register_use(&self, id: &IdentityId) -> Result<(), IdentitiesError> {
        self.bus
            .identities_proxy()
            .await?
            .register_use(id.as_str())
            .await
            .map_err(|e| identities_error(&e))
    }

    /// The apps that signed in with `id`. Only Settings and the shell may ask.
    pub async fn uses(&self, id: &IdentityId) -> Result<Vec<AppName>, IdentitiesError> {
        self.bus
            .identities_proxy()
            .await?
            .uses(id.as_str())
            .await
            .map_err(|e| identities_error(&e))?
            .iter()
            .map(|name| AppName::parse(name).map_err(|_| malformed(Malformed::Id)))
            .collect()
    }

    /// The identities removed from now on. It subscribes, then reads the list once so accountd
    /// counts this connection among those it tells.
    pub async fn watch_removals(&self) -> Result<Changes<IdentityId>, IdentitiesError> {
        let proxy = self.bus.identities_proxy().await?;
        let stream = proxy
            .receive_identity_removed()
            .await
            .map_err(|e| identities_error(&e))?;
        // Reading the list once is how accountd counts this connection among those it tells.
        proxy.list().await.map_err(|e| identities_error(&e))?;
        Ok(Changes::new(stream.map(removed)))
    }
}
