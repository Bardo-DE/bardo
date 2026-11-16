//! `org.bluez.Device1`.

use zbus::proxy;

/// A device, at `/org/bluez/hciN/dev_XX_XX_XX_XX_XX_XX`.
#[proxy(interface = "org.bluez.Device1", default_service = "org.bluez")]
pub trait Device1 {
    /// Connects every profile the device offers that is set to connect on its own.
    fn connect(&self) -> zbus::Result<()>;

    /// Disconnects every profile.
    fn disconnect(&self) -> zbus::Result<()>;

    /// Pairs, asking the program's registered agent for anything the person must confirm.
    fn pair(&self) -> zbus::Result<()>;

    /// The device's address, `XX:XX:XX:XX:XX:XX`.
    #[zbus(property)]
    fn address(&self) -> zbus::Result<String>;

    /// The name to show: the person's own name for it, else the device's, else its address.
    #[zbus(property)]
    fn alias(&self) -> zbus::Result<String>;

    /// A freedesktop icon name for its kind (`audio-headphones`, `input-keyboard`, ...); empty
    /// when the device does not say.
    #[zbus(property)]
    fn icon(&self) -> zbus::Result<String>;

    /// Whether it is paired.
    #[zbus(property)]
    fn paired(&self) -> zbus::Result<bool>;

    /// Whether it is connected.
    #[zbus(property)]
    fn connected(&self) -> zbus::Result<bool>;

    /// Lets the device connect without asking again.
    #[zbus(property)]
    fn set_trusted(&self, value: bool) -> zbus::Result<()>;
}
