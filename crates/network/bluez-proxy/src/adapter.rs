//! `org.bluez.Adapter1`.

use zbus::proxy;
use zbus::zvariant::ObjectPath;

/// An adapter, at `/org/bluez/hciN`.
#[proxy(interface = "org.bluez.Adapter1", default_service = "org.bluez")]
pub trait Adapter1 {
    /// Starts looking for devices nearby.
    fn start_discovery(&self) -> zbus::Result<()>;

    /// Stops looking.
    fn stop_discovery(&self) -> zbus::Result<()>;

    /// Forgets a device: its pairing and the object that stands for it.
    fn remove_device(&self, device: &ObjectPath<'_>) -> zbus::Result<()>;

    /// Whether the radio is on.
    #[zbus(property)]
    fn powered(&self) -> zbus::Result<bool>;

    /// Turns the radio on or off.
    #[zbus(property)]
    fn set_powered(&self, value: bool) -> zbus::Result<()>;
}
