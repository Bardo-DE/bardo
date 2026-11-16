//! Typed `zbus` proxies for `NetworkManager`'s D-Bus API (`org.freedesktop.NetworkManager`), one
//! file per interface, written from the documented API so nothing here depends on a third-party
//! binding crate or on libdbus.
//!
//! Each proxy names the members a desktop's network menu and settings page read and call, not the
//! whole interface. The numbers `NetworkManager` sends for kinds and states decode to the enums in
//! [`kind`], each with an `Other` that keeps a value this crate does not name.

mod access_point;
mod active;
mod device;
pub mod kind;
mod manager;
mod settings;
mod wireless;

pub use access_point::AccessPointProxy;
pub use active::ActiveConnectionProxy;
pub use device::DeviceProxy;
pub use kind::{ActiveState, Connectivity, DeviceState, DeviceType};
pub use manager::NetworkManagerProxy;
pub use settings::{ConnectionMap, ConnectionSettingsProxy, ConnectionToAdd, SettingsProxy};
pub use wireless::WirelessDeviceProxy;

/// The bus name `NetworkManager` owns, on the system bus.
pub const SERVICE: &str = "org.freedesktop.NetworkManager";
