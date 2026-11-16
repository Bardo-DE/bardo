//! The numbers `NetworkManager` sends for kinds and states, as enums. Each has an `Other` that keeps
//! a value this crate does not name, so a newer `NetworkManager` never decodes to a wrong variant,
//! and `to_wire` gives back the number it came from.

/// `NMDeviceType`: what kind of device it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DeviceType {
    /// 0: not known yet.
    Unknown,
    /// 1: wired Ethernet.
    Ethernet,
    /// 2: Wi-Fi.
    Wifi,
    /// 5: a Bluetooth network (a phone's tethering).
    Bluetooth,
    /// 8: a mobile broadband modem.
    Modem,
    /// 13: a bridge.
    Bridge,
    /// 14: a generic or virtual device, such as a TUN interface `NetworkManager` does not manage.
    Generic,
    /// 16: a TUN or TAP interface.
    Tun,
    /// 29: a `WireGuard` tunnel.
    WireGuard,
    /// 32: the loopback interface.
    Loopback,
    /// Any other value.
    Other(u32),
}

impl DeviceType {
    const WIRE: &[(Self, u32)] = &[
        (Self::Unknown, 0),
        (Self::Ethernet, 1),
        (Self::Wifi, 2),
        (Self::Bluetooth, 5),
        (Self::Modem, 8),
        (Self::Bridge, 13),
        (Self::Generic, 14),
        (Self::Tun, 16),
        (Self::WireGuard, 29),
        (Self::Loopback, 32),
    ];

    /// The kind for a `DeviceType` value.
    #[must_use]
    pub fn from_wire(value: u32) -> Self {
        named(Self::WIRE, value).unwrap_or(Self::Other(value))
    }

    /// The `DeviceType` value for this kind.
    #[must_use]
    pub fn to_wire(self) -> u32 {
        match self {
            Self::Other(value) => value,
            named_kind => wire_of(Self::WIRE, named_kind),
        }
    }
}

/// `NMDeviceState`: where a device is on its way up or down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum DeviceState {
    /// 0: not known.
    #[default]
    Unknown,
    /// 10: `NetworkManager` does not manage it.
    Unmanaged,
    /// 20: managed, but it cannot be used (no carrier, Wi-Fi switched off, firmware missing).
    Unavailable,
    /// 30: ready, with no connection.
    Disconnected,
    /// 40: preparing to connect.
    Prepare,
    /// 50: connecting (associating with a Wi-Fi network).
    Config,
    /// 60: waiting for a secret, such as a Wi-Fi password.
    NeedAuth,
    /// 70: asking for addresses.
    IpConfig,
    /// 80: checking that the network reaches further.
    IpCheck,
    /// 90: waiting for a connection it depends on.
    Secondaries,
    /// 100: connected.
    Activated,
    /// 110: disconnecting.
    Deactivating,
    /// 120: the connection failed.
    Failed,
    /// Any other value.
    Other(u32),
}

impl DeviceState {
    const WIRE: &[(Self, u32)] = &[
        (Self::Unknown, 0),
        (Self::Unmanaged, 10),
        (Self::Unavailable, 20),
        (Self::Disconnected, 30),
        (Self::Prepare, 40),
        (Self::Config, 50),
        (Self::NeedAuth, 60),
        (Self::IpConfig, 70),
        (Self::IpCheck, 80),
        (Self::Secondaries, 90),
        (Self::Activated, 100),
        (Self::Deactivating, 110),
        (Self::Failed, 120),
    ];

    /// The state for a `State` value.
    #[must_use]
    pub fn from_wire(value: u32) -> Self {
        named(Self::WIRE, value).unwrap_or(Self::Other(value))
    }

    /// The `State` value for this state.
    #[must_use]
    pub fn to_wire(self) -> u32 {
        match self {
            Self::Other(value) => value,
            named_state => wire_of(Self::WIRE, named_state),
        }
    }
}

/// `NMActiveConnectionState`: where an active connection is on its way up or down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ActiveState {
    /// 0: not known.
    #[default]
    Unknown,
    /// 1: coming up.
    Activating,
    /// 2: up.
    Activated,
    /// 3: going down.
    Deactivating,
    /// 4: down.
    Deactivated,
    /// Any other value.
    Other(u32),
}

impl ActiveState {
    const WIRE: &[(Self, u32)] = &[
        (Self::Unknown, 0),
        (Self::Activating, 1),
        (Self::Activated, 2),
        (Self::Deactivating, 3),
        (Self::Deactivated, 4),
    ];

    /// The state for a `State` value.
    #[must_use]
    pub fn from_wire(value: u32) -> Self {
        named(Self::WIRE, value).unwrap_or(Self::Other(value))
    }

    /// The `State` value for this state.
    #[must_use]
    pub fn to_wire(self) -> u32 {
        match self {
            Self::Other(value) => value,
            named_state => wire_of(Self::WIRE, named_state),
        }
    }
}

/// `NMConnectivityState`: how far the network reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Connectivity {
    /// 0: not checked, or checking is off.
    #[default]
    Unknown,
    /// 1: no network.
    None,
    /// 2: a sign-in page stands between the network and the internet.
    Portal,
    /// 3: a network, but not the internet.
    Limited,
    /// 4: the internet.
    Full,
    /// Any other value.
    Other(u32),
}

impl Connectivity {
    const WIRE: &[(Self, u32)] = &[
        (Self::Unknown, 0),
        (Self::None, 1),
        (Self::Portal, 2),
        (Self::Limited, 3),
        (Self::Full, 4),
    ];

    /// The reach for a `Connectivity` value.
    #[must_use]
    pub fn from_wire(value: u32) -> Self {
        named(Self::WIRE, value).unwrap_or(Self::Other(value))
    }

    /// The `Connectivity` value for this reach.
    #[must_use]
    pub fn to_wire(self) -> u32 {
        match self {
            Self::Other(value) => value,
            named_reach => wire_of(Self::WIRE, named_reach),
        }
    }
}

fn named<T: Copy>(table: &[(T, u32)], value: u32) -> Option<T> {
    table
        .iter()
        .find(|(_, wire)| *wire == value)
        .map(|(named, _)| *named)
}

/// Every named variant is in its table, so the fallback is never reached.
fn wire_of<T: Copy + PartialEq>(table: &[(T, u32)], named: T) -> u32 {
    table
        .iter()
        .find(|(variant, _)| *variant == named)
        .map_or(0, |(_, wire)| *wire)
}
