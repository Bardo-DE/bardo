//! The caller's side of `org.bardo.Memory1.Control`, the members Settings uses: memory-dbus's
//! proxy, built for the owner that was confirmed to be the memory service.

use crate::bus::Pinned;
use memory_dbus::ControlProxy;

/// The controls of the memory service whose owner was confirmed: calls go to that owner by its
/// unique name.
///
/// # Errors
///
/// The `zbus::Error` when the proxy cannot be built for that owner.
pub async fn control(memory: &Pinned) -> zbus::Result<ControlProxy<'_>> {
    ControlProxy::builder(&memory.connection)
        .destination(memory.destination())?
        .build()
        .await
}
