//! `org.freedesktop.NetworkManager.Settings` and `.Settings.Connection`.

use std::collections::HashMap;

use zbus::proxy;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

/// A saved connection's settings as `NetworkManager` answers them, by group (`connection`,
/// `802-11-wireless`, `ipv4`, ...) and key.
pub type ConnectionMap = HashMap<String, HashMap<String, OwnedValue>>;

/// The settings of a connection to be saved, by group and key.
pub type ConnectionToAdd = HashMap<String, HashMap<String, Value<'static>>>;

/// The saved connections, at `/org/freedesktop/NetworkManager/Settings`.
#[proxy(
    interface = "org.freedesktop.NetworkManager.Settings",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Settings"
)]
pub trait Settings {
    /// Every saved connection.
    #[zbus(property)]
    fn connections(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    /// A connection was saved.
    #[zbus(signal)]
    fn new_connection(&self, connection: OwnedObjectPath) -> zbus::Result<()>;

    /// A saved connection was deleted.
    #[zbus(signal)]
    fn connection_removed(&self, connection: OwnedObjectPath) -> zbus::Result<()>;
}

/// One saved connection, at `/org/freedesktop/NetworkManager/Settings/N`.
#[proxy(
    interface = "org.freedesktop.NetworkManager.Settings.Connection",
    default_service = "org.freedesktop.NetworkManager"
)]
pub trait ConnectionSettings {
    /// Its settings, without secrets.
    fn get_settings(&self) -> zbus::Result<ConnectionMap>;

    /// Forgets it.
    fn delete(&self) -> zbus::Result<()>;
}
