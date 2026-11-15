//! Typed `zbus` proxies for BlueZ's D-Bus API (`org.bluez`), written from BlueZ's documented
//! interfaces. Not `bluer` or `bluez-async`: both run on the `dbus` crate and link libdbus, a
//! second D-Bus stack beside `zbus`.
//!
//! Each proxy names the members a desktop's Bluetooth menu and settings page read and call, not
//! the whole interface. A program that pairs serves its own `org.bluez.Agent1` and refuses with
//! [`AgentError::Rejected`].

mod adapter;
mod agent;
mod battery;
mod device;

pub use adapter::Adapter1Proxy;
pub use agent::{AgentError, AgentManager1Proxy, Capability};
pub use battery::Battery1Proxy;
pub use device::Device1Proxy;

use std::collections::HashMap;
use zbus::names::OwnedInterfaceName;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

/// The bus name BlueZ owns, on the system bus.
pub const SERVICE: &str = "org.bluez";

/// What `org.freedesktop.DBus.ObjectManager.GetManagedObjects` on BlueZ's `/` answers: every
/// adapter and device, its interfaces and their properties.
pub type Objects =
    HashMap<OwnedObjectPath, HashMap<OwnedInterfaceName, HashMap<String, OwnedValue>>>;
