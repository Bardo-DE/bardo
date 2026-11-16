//! `org.bluez.Battery1`.

use zbus::proxy;

/// A device's battery, on the device's own path, present only once `BlueZ` has a reading.
#[proxy(interface = "org.bluez.Battery1", default_service = "org.bluez")]
pub trait Battery1 {
    /// Charge, 0 to 100.
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<u8>;
}
