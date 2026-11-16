//! `org.freedesktop.NetworkManager`.

use zbus::proxy;
use zbus::zvariant::{ObjectPath, OwnedObjectPath};

use crate::settings::ConnectionToAdd;

/// The manager, at `/org/freedesktop/NetworkManager`.
#[proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
pub trait NetworkManager {
    /// Every device, including those `NetworkManager` does not manage.
    fn get_all_devices(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    /// Activates a saved connection on a device; `specific_object` is an access point for Wi-Fi,
    /// or `/` for none. Answers the active connection.
    fn activate_connection(
        &self,
        connection: &ObjectPath<'_>,
        device: &ObjectPath<'_>,
        specific_object: &ObjectPath<'_>,
    ) -> zbus::Result<OwnedObjectPath>;

    /// Saves a new connection and activates it. Answers the saved connection and the active one.
    fn add_and_activate_connection(
        &self,
        connection: ConnectionToAdd,
        device: &ObjectPath<'_>,
        specific_object: &ObjectPath<'_>,
    ) -> zbus::Result<(OwnedObjectPath, OwnedObjectPath)>;

    /// A device appeared.
    #[zbus(signal)]
    fn device_added(&self, device: OwnedObjectPath) -> zbus::Result<()>;

    /// A device went away.
    #[zbus(signal)]
    fn device_removed(&self, device: OwnedObjectPath) -> zbus::Result<()>;

    /// The active connection that carries the default route, or `/`.
    #[zbus(property)]
    fn primary_connection(&self) -> zbus::Result<OwnedObjectPath>;

    /// The active connection that will become primary once it is up, or `/`.
    #[zbus(property)]
    fn activating_connection(&self) -> zbus::Result<OwnedObjectPath>;

    /// How far the network reaches, as `NMConnectivityState` ([`crate::Connectivity`]).
    #[zbus(property)]
    fn connectivity(&self) -> zbus::Result<u32>;

    /// Wi-Fi's software switch.
    #[zbus(property)]
    fn wireless_enabled(&self) -> zbus::Result<bool>;

    /// Turns Wi-Fi's software switch on or off.
    #[zbus(property)]
    fn set_wireless_enabled(&self, value: bool) -> zbus::Result<()>;

    /// Wi-Fi's hardware switch (a key or a slider on the machine); read only.
    #[zbus(property)]
    fn wireless_hardware_enabled(&self) -> zbus::Result<bool>;
}
