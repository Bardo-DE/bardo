//! A private dbus-daemon for tests that serve or listen on a bus.
//!
//! The repo's `dev/private-bus.conf`, no environment, no service directories, killed and reaped
//! when dropped, so a failing test leaves nothing behind. Nothing here reaches the person's real
//! session bus or the system bus. This crate's own tests and `compositor-testkit` share it.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use zbus::Connection;

/// A running private bus.
#[derive(Debug)]
pub struct PrivateBus {
    daemon: Child,
    /// Where clients connect.
    pub address: String,
}

impl PrivateBus {
    /// Starts the daemon and reads the address it listens on; panics when it cannot (tests only).
    ///
    /// # Panics
    ///
    /// Panics when `dbus-daemon` cannot be started or prints no address.
    #[must_use]
    pub fn start() -> PrivateBus {
        let config = concat!(env!("CARGO_MANIFEST_DIR"), "/../dev/private-bus.conf");
        let daemon = Command::new("dbus-daemon")
            .env_clear()
            .args(["--config-file", config, "--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("dbus-daemon");
        let mut bus = PrivateBus {
            daemon,
            address: String::new(),
        };
        let stdout = bus.daemon.stdout.take().expect("stdout");
        BufReader::new(stdout)
            .read_line(&mut bus.address)
            .expect("address");
        bus.address = bus.address.trim().to_owned();
        assert!(!bus.address.is_empty(), "dbus-daemon printed no address");
        bus
    }

    /// A new client connection on this bus.
    ///
    /// # Panics
    ///
    /// Panics when the connection cannot be made.
    pub async fn client(&self) -> Connection {
        zbus::connection::Builder::address(self.address.as_str())
            .expect("address")
            .build()
            .await
            .expect("client")
    }
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
    }
}

/// The unique bus name of `connection`.
///
/// # Panics
///
/// Panics when `connection` has no unique name.
#[must_use]
pub fn name_of(connection: &Connection) -> String {
    connection
        .unique_name()
        .map(|name| name.as_str().to_owned())
        .expect("a unique name")
}
