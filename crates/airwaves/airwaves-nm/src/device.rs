//! `org.freedesktop.NetworkManager.Device`.

use zbus::proxy;

/// A device, at `/org/freedesktop/NetworkManager/Devices/N`.
#[proxy(
    interface = "org.freedesktop.NetworkManager.Device",
    default_service = "org.freedesktop.NetworkManager"
)]
pub trait Device {
    /// Takes the device's active connection down and keeps it from coming back on its own.
    fn disconnect(&self) -> zbus::Result<()>;

    /// The kind, as `NMDeviceType` ([`crate::DeviceType`]).
    #[zbus(property)]
    fn device_type(&self) -> zbus::Result<u32>;

    /// The state, as `NMDeviceState` ([`crate::DeviceState`]).
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;

    /// The state changed, and why (`NMDeviceStateReason`). Named apart from the `State`
    /// property, whose change stream would take the same name.
    #[zbus(signal, name = "StateChanged")]
    fn state_change(&self, new_state: u32, old_state: u32, reason: u32) -> zbus::Result<()>;
}
