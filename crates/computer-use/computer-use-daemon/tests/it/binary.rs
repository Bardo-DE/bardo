//! The `computer_use_daemon` binary on a private bus with a cleared environment and no compositor: it starts,
//! owns both of its names, reports the backend as unavailable, leaves `computer_use.enabled` off, and
//! stops cleanly on SIGTERM and when the bus goes away.

use crate::support::bus::PrivateBus;
use computer_use_dbus::ManagerProxy;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use zbus::fdo;
use zbus::names::BusName;

/// A running `computer_use_daemon`, killed by PID if the test ends before it does.
struct Daemon(Child);

impl Daemon {
    fn start(bus: &PrivateBus, home: &Path, extra: &[(&str, &str)]) -> Self {
        let child = Command::new(env!("CARGO_BIN_EXE_bardo-computer-use"))
            .env_clear()
            .env("DBUS_SESSION_BUS_ADDRESS", bus.address())
            .env("HOME", home)
            .env("XDG_RUNTIME_DIR", home)
            .env("XDG_CONFIG_HOME", home.join("config"))
            .envs(extra.iter().copied())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("computer_use_daemon starts");
        Self(child)
    }

    fn pid(&self) -> u32 {
        self.0.id()
    }

    fn wait(&mut self) -> ExitStatus {
        self.0.wait().expect("computer_use_daemon exits")
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Waits until `name` has an owner on the bus: subscribes first, then asks, so it is never
/// missed.
async fn owned(connection: &zbus::Connection, name: &str) {
    use futures_util::StreamExt;
    let dbus = fdo::DBusProxy::new(connection).await.expect("dbus");
    let mut changes = dbus.receive_name_owner_changed().await.expect("signal");
    let bus_name = BusName::try_from(name).expect("a name");
    while !dbus.name_has_owner(bus_name.clone()).await.expect("ask") {
        changes.next().await.expect("the bus is up");
    }
}

fn sigterm(pid: u32) {
    let status = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status()
        .expect("kill");
    assert!(status.success());
}

#[tokio::test]
async fn computer_use_daemon_without_a_compositor_serves_its_names_and_says_the_backend_is_none() {
    let dir = tempfile::tempdir().expect("dir");
    let bus = PrivateBus::start(dir.path());
    let mut daemon = Daemon::start(&bus, dir.path(), &[]);
    let me = bus.connect().await;
    owned(&me, "org.bardo.ComputerUse1").await;
    owned(&me, "org.bardo.ComputerUse").await;

    let manager = ManagerProxy::new(&me).await.expect("manager");
    assert_eq!(manager.backend().await.expect("backend"), "none");
    assert!(manager.modes().await.expect("modes").is_empty());
    assert!(manager.runs().await.expect("runs").is_empty());

    sigterm(daemon.pid());
    assert!(daemon.wait().success(), "SIGTERM is a clean stop");
}

#[tokio::test]
async fn computer_use_daemon_stops_when_the_bus_goes_away() {
    let dir = tempfile::tempdir().expect("dir");
    let bus = PrivateBus::start(dir.path());
    let mut daemon = Daemon::start(&bus, dir.path(), &[]);
    let me = bus.connect().await;
    owned(&me, "org.bardo.ComputerUse1").await;
    drop(me);
    drop(bus);
    assert!(daemon.wait().success(), "a closed bus is a clean stop");
}

#[tokio::test]
async fn a_bad_configuration_is_a_start_failure_not_a_panic() {
    let dir = tempfile::tempdir().expect("dir");
    let bus = PrivateBus::start(dir.path());
    let config = dir.path().join("computer-use.toml");
    std::fs::write(&config, "settings = 3").expect("config");
    let mut daemon = Daemon::start(
        &bus,
        dir.path(),
        &[("BARDO_COMPUTER_USE_CONFIG", config.to_str().unwrap())],
    );
    let status = daemon.wait();
    assert_eq!(status.code(), Some(2));
}

#[tokio::test]
async fn a_missing_configuration_file_means_the_defaults() {
    let dir = tempfile::tempdir().expect("dir");
    let bus = PrivateBus::start(dir.path());
    let missing = dir.path().join("nothing.toml");
    let mut daemon = Daemon::start(
        &bus,
        dir.path(),
        &[("BARDO_COMPUTER_USE_CONFIG", missing.to_str().unwrap())],
    );
    let me = bus.connect().await;
    owned(&me, "org.bardo.ComputerUse1").await;
    sigterm(daemon.pid());
    assert!(daemon.wait().success());
}
