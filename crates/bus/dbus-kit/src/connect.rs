//! Which bus a program connects to, and the connection.
//!
//! A daemon takes a [`BusTarget`] in its configuration instead of reading
//! `DBUS_SESSION_BUS_ADDRESS` itself, so a program that runs the daemon inside it (a test on a
//! private bus, a host that embeds it) says where the bus is.

use zbus::Connection;
use zbus::connection::Builder;

/// The bus to connect to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BusTarget {
    /// The person's session bus, found the usual way.
    Session,
    /// The computer's system bus, found the usual way.
    System,
    /// The bus listening at this address (`unix:path=...`).
    Address(String),
}

impl BusTarget {
    /// A connection to the bus.
    ///
    /// # Errors
    ///
    /// The address is not one, or the bus cannot be reached.
    pub async fn connect(&self) -> zbus::Result<Connection> {
        match self {
            Self::Session => Builder::session()?.build().await,
            Self::System => Builder::system()?.build().await,
            Self::Address(address) => Builder::address(address.as_str())?.build().await,
        }
    }
}

/// The session bus a daemon serves on.
///
/// It is `$DBUS_SESSION_BUS_ADDRESS`, else the address of the bus that started this process by
/// activation (`$DBUS_STARTER_ADDRESS`), else the default per-user socket. `env` reads the
/// environment, so a test names its own bus.
///
/// # Errors
///
/// The D-Bus error of an address that cannot be parsed or a connection that cannot be made.
pub async fn session_from_env(env: &impl Fn(&str) -> Option<String>) -> zbus::Result<Connection> {
    let address = env("DBUS_SESSION_BUS_ADDRESS")
        .or_else(|| env("DBUS_STARTER_ADDRESS"))
        .filter(|a| !a.is_empty());
    match address {
        Some(address) => BusTarget::Address(address).connect().await,
        None => BusTarget::Session.connect().await,
    }
}
