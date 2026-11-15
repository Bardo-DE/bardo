//! `org.freedesktop.NetworkManager.Device.Wireless`.

use std::collections::HashMap;

use zbus::proxy;
use zbus::zvariant::{OwnedObjectPath, Value};

/// A Wi-Fi device, on the same path as its [`crate::DeviceProxy`].
#[proxy(
    interface = "org.freedesktop.NetworkManager.Device.Wireless",
    default_service = "org.freedesktop.NetworkManager"
)]
pub trait WirelessDevice {
    /// Asks for a scan; `options` may name `ssids` to probe for hidden networks.
    fn request_scan(&self, options: HashMap<String, Value<'_>>) -> zbus::Result<()>;

    /// The access points the last scan saw.
    #[zbus(property)]
    fn access_points(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    /// The access point in use, or `/`.
    #[zbus(property)]
    fn active_access_point(&self) -> zbus::Result<OwnedObjectPath>;

    /// When the last scan finished, in `CLOCK_BOOTTIME` milliseconds; -1 for never.
    #[zbus(property)]
    fn last_scan(&self) -> zbus::Result<i64>;

    /// A scan saw a new access point.
    #[zbus(signal)]
    fn access_point_added(&self, access_point: OwnedObjectPath) -> zbus::Result<()>;

    /// An access point is no longer seen.
    #[zbus(signal)]
    fn access_point_removed(&self, access_point: OwnedObjectPath) -> zbus::Result<()>;
}
