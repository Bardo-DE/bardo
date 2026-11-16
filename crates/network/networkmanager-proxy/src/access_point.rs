//! `org.freedesktop.NetworkManager.AccessPoint`.

use zbus::proxy;

/// An access point, at `/org/freedesktop/NetworkManager/AccessPoint/N`.
#[proxy(
    interface = "org.freedesktop.NetworkManager.AccessPoint",
    default_service = "org.freedesktop.NetworkManager"
)]
pub trait AccessPoint {
    /// The network's name, as bytes (not always UTF-8).
    #[zbus(property)]
    fn ssid(&self) -> zbus::Result<Vec<u8>>;

    /// Signal strength, 0 to 100.
    #[zbus(property)]
    fn strength(&self) -> zbus::Result<u8>;

    /// `NM80211ApFlags` (bit 0: privacy).
    #[zbus(property)]
    fn flags(&self) -> zbus::Result<u32>;

    /// `NM80211ApSecurityFlags` for WPA.
    #[zbus(property)]
    fn wpa_flags(&self) -> zbus::Result<u32>;

    /// `NM80211ApSecurityFlags` for WPA2 and WPA3 (RSN).
    #[zbus(property)]
    fn rsn_flags(&self) -> zbus::Result<u32>;
}
