//! `org.freedesktop.NetworkManager.Connection.Active`.

use zbus::proxy;
use zbus::zvariant::OwnedObjectPath;

/// An active connection, at `/org/freedesktop/NetworkManager/ActiveConnection/N`.
#[proxy(
    interface = "org.freedesktop.NetworkManager.Connection.Active",
    default_service = "org.freedesktop.NetworkManager"
)]
pub trait ActiveConnection {
    /// The state, as `NMActiveConnectionState` ([`crate::ActiveState`]).
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;

    /// The connection type, such as `802-11-wireless` (D-Bus property `Type`).
    #[zbus(property, name = "Type")]
    fn kind(&self) -> zbus::Result<String>;

    /// The devices it runs on.
    #[zbus(property)]
    fn devices(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
}
