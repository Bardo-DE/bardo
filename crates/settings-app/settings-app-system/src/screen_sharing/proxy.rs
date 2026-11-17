//! The caller's side of `org.bardo.Portal1`, the members only the Settings app may call: the
//! portal's own proxy, built for the owner that was confirmed.

use crate::bus::Pinned;
use portal_dbus::PortalProxy;

/// The portal whose owner was confirmed: calls go to that owner by its unique name.
pub async fn portal(running: &Pinned) -> zbus::Result<PortalProxy<'_>> {
    PortalProxy::builder(&running.connection)
        .destination(running.destination())?
        .build()
        .await
}
