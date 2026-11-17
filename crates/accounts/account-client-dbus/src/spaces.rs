//! The desktop-wide Spaces an app's own Space can link to (`org.bardo.Spaces1` on accountd).
//!
//! An app keeps its own Spaces (Bardo's Spaces kit) and stores, per Space, an optional link: the
//! id of a desktop-wide Space from this list. It makes the `SpaceId` it hands memory and consent
//! with `account_core::SpaceId::for_app`. When a desktop-wide Space is removed, the watch says so
//! and the app falls back to its own Space; that is the app's choice.
//!
//! ```ignore
//! let spaces = bus.spaces();
//! let mut changes = spaces.watch().await?;          // subscribe before reading the list
//! let list = spaces.list().await?;
//! let id = spaces.create(&SpaceName::parse("Work")?, &look).await?;
//! while let Some(change) = changes.next().await { /* (id, SpaceChange) */ }
//! ```

use crate::Bus;
use crate::failure::{link_error, spaces_error};
use account_client::{Changes, LinkError, Malformed, MalformedError, SpacesError};
use account_core::{DesktopSpace, DesktopSpaceRecord, SpaceChange, SpaceLook, SpaceName};
use account_dbus::Details;
use account_dbus::accounts::{
    SPACE_KEY_CREATED, SPACE_KEY_LOOK, SPACE_KEY_NAME, SpaceChanged, SpacesProxy,
};
use base_ids::word::from_word;
use base_time::UnixSeconds;
use futures_util::StreamExt;
use zbus::zvariant::OwnedValue;

/// accountd's registry of desktop-wide Spaces, as an app uses it over the bus.
#[derive(Debug, Clone)]
pub struct Spaces {
    bus: Bus,
}

impl Bus {
    /// The handle on spaces.
    #[must_use]
    pub fn spaces(&self) -> Spaces {
        Spaces { bus: self.clone() }
    }
}

impl Bus {
    async fn spaces_proxy(&self) -> Result<SpacesProxy<'static>, SpacesError> {
        self.accounts
            .proxy(&self.connection)
            .await
            .map_err(|e| SpacesError::Link(link_error(&e)))
    }
}

fn malformed(what: impl Into<MalformedError>) -> SpacesError {
    SpacesError::Link(LinkError::Malformed(what.into()))
}

fn field<'a>(details: &'a Details, key: &str) -> Result<&'a OwnedValue, SpacesError> {
    details.get(key).ok_or_else(|| malformed(Malformed::Row))
}

fn text(details: &Details, key: &str) -> Result<String, SpacesError> {
    let value = field(details, key)?
        .try_clone()
        .map_err(|e| malformed(Malformed::Row.because(e)))?;
    String::try_from(value).map_err(|e| malformed(Malformed::Row.because(e)))
}

fn record(id: &str, details: &Details) -> Result<DesktopSpaceRecord, SpacesError> {
    let created = field(details, SPACE_KEY_CREATED)?
        .try_clone()
        .map_err(|e| malformed(Malformed::Row.because(e)))?;
    let created = i64::try_from(created).map_err(|e| malformed(Malformed::Row.because(e)))?;
    Ok(DesktopSpaceRecord {
        id: DesktopSpace::parse(id).map_err(|e| malformed(Malformed::Id.because(e)))?,
        name: SpaceName::parse(&text(details, SPACE_KEY_NAME)?)
            .map_err(|e| malformed(Malformed::Row.because(e)))?,
        look: SpaceLook::parse(&text(details, SPACE_KEY_LOOK)?)
            .map_err(|e| malformed(Malformed::Row.because(e)))?,
        created: UnixSeconds(created),
    })
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "a stream `map` callback, which is handed the signal by value"
)]
fn change(signal: SpaceChanged) -> Result<(DesktopSpace, SpaceChange), MalformedError> {
    let args = signal.args().map_err(|e| Malformed::Signal.because(e))?;
    let what = from_word::<SpaceChange>(args.what).ok_or(Malformed::Signal)?;
    Ok((
        DesktopSpace::parse(args.id).map_err(|e| Malformed::Id.because(e))?,
        what,
    ))
}

impl Spaces {
    /// Every desktop-wide Space, in the order they were made.
    ///
    /// # Errors
    ///
    /// [`SpacesError`] when the call to accountd fails or a row of its reply is malformed.
    pub async fn list(&self) -> Result<Vec<DesktopSpaceRecord>, SpacesError> {
        self.bus
            .spaces_proxy()
            .await?
            .list()
            .await
            .map_err(|e| spaces_error(&e))?
            .iter()
            .map(|(id, details)| record(id, details))
            .collect()
    }

    /// Makes a desktop-wide Space and answers its id, which a rename keeps. accountd lets an app
    /// make only a few a minute (ten); more is [`SpacesError::TooMany`].
    ///
    /// # Errors
    ///
    /// [`SpacesError::TooMany`] when this app made too many Spaces lately, or another
    /// [`SpacesError`] when the call fails.
    pub async fn create(
        &self,
        name: &SpaceName,
        look: &SpaceLook,
    ) -> Result<DesktopSpace, SpacesError> {
        let id = self
            .bus
            .spaces_proxy()
            .await?
            .create(name.as_str(), look.as_str())
            .await
            .map_err(|e| spaces_error(&e))?;
        DesktopSpace::parse(&id).map_err(|e| malformed(Malformed::Id.because(e)))
    }

    /// Registers a Space whose id the caller minted (the compositor, per workspace); one already
    /// registered is left as it is. Only the compositor, Settings and the shell may.
    ///
    /// # Errors
    ///
    /// [`SpacesError`] when the caller may not register Spaces or the call fails.
    pub async fn register(
        &self,
        id: &DesktopSpace,
        name: &SpaceName,
        look: &SpaceLook,
    ) -> Result<(), SpacesError> {
        self.bus
            .spaces_proxy()
            .await?
            .register(id.as_str(), name.as_str(), look.as_str())
            .await
            .map_err(|e| spaces_error(&e))
    }

    /// Renames a Space. Only Settings and the shell may.
    ///
    /// # Errors
    ///
    /// [`SpacesError`] when the caller may not rename Spaces or the call fails.
    pub async fn rename(&self, id: &DesktopSpace, name: &SpaceName) -> Result<(), SpacesError> {
        self.bus
            .spaces_proxy()
            .await?
            .rename(id.as_str(), name.as_str())
            .await
            .map_err(|e| spaces_error(&e))
    }

    /// Gives a Space a new look. Only Settings and the shell may.
    ///
    /// # Errors
    ///
    /// [`SpacesError`] when the caller may not change a Space's look or the call fails.
    pub async fn set_look(&self, id: &DesktopSpace, look: &SpaceLook) -> Result<(), SpacesError> {
        self.bus
            .spaces_proxy()
            .await?
            .set_look(id.as_str(), look.as_str())
            .await
            .map_err(|e| spaces_error(&e))
    }

    /// Removes a Space. Only Settings and the shell may.
    ///
    /// # Errors
    ///
    /// [`SpacesError`] when the caller may not remove Spaces or the call fails.
    pub async fn remove(&self, id: &DesktopSpace) -> Result<(), SpacesError> {
        self.bus
            .spaces_proxy()
            .await?
            .remove(id.as_str())
            .await
            .map_err(|e| spaces_error(&e))
    }

    /// The changes to the Spaces, from now on: which Space, and what became of it. It subscribes,
    /// then calls `List` once so accountd counts this connection among those it tells.
    ///
    /// # Errors
    ///
    /// [`SpacesError`] when subscribing or the first `List` call fails.
    pub async fn watch(&self) -> Result<Changes<(DesktopSpace, SpaceChange)>, SpacesError> {
        let proxy = self.bus.spaces_proxy().await?;
        let stream = proxy
            .receive_changed()
            .await
            .map_err(|e| spaces_error(&e))?;
        // Reading the list once is how accountd counts this connection among those it tells.
        proxy.list().await.map_err(|e| spaces_error(&e))?;
        Ok(Changes::new(stream.map(change)))
    }
}
