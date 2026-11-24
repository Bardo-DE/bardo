//! bardo-shell's Bluetooth service against `dev/fake-bluez.py` on a private bus, never the system bus:
//! bardo-shell starts first, `BlueZ` appears late, is killed, and comes back. The test needs a
//! `dbus-daemon` and python's `dbus` and `gi`, and skips, saying so, without them.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use shell_model::bluetooth::BluetoothService;
use shell_model::bluetooth::{Bluetooth, BtPower};
use shell_model::units::Presence;
use shell_services::env::{BusRoute, Buses, Env};
use shell_services::service::spawn;
use base_testkit::PrivateBus;
use tokio::sync::watch;

fn fake_bluez() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dev/fake-bluez.py")
}

/// The fake started on `bus`, once it has printed that it owns the name. Panics, saying what to
/// install, when python cannot run it.
fn start_fake(bus: &PrivateBus, log: &Path) -> Child {
    let mut child = Command::new("python3")
        .arg(fake_bluez())
        .arg(bus.address())
        .arg(log)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("python3 is a required test tool for fake-bluez.py: {e}"));
    let stdout = child
        .stdout
        .take()
        .unwrap_or_else(|| panic!("fake-bluez.py has no stdout"));
    let mut line = String::new();
    std::io::BufRead::read_line(&mut std::io::BufReader::new(stdout), &mut line)
        .unwrap_or_else(|e| panic!("reading fake-bluez.py's first line: {e}"));
    if line.starts_with("fake-bluez: org.bluez") {
        child
    } else {
        let _ = child.kill();
        let _ = child.wait();
        panic!(
            "fake-bluez.py cannot run (python3 with the dbus and gi modules is a required test tool): {line:?}"
        );
    }
}

fn stop(mut child: Child) {
    let _ = child.kill();
    let _ = child.wait();
}

async fn until(
    rx: &mut watch::Receiver<Bluetooth>,
    what: &str,
    ok: impl FnMut(&Bluetooth) -> bool,
) {
    let waited = tokio::time::timeout(Duration::from_secs(10), rx.wait_for(ok))
        .await
        .map(|r| r.map(|_| ()));
    match waited {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("{what}: {e}"),
        Err(elapsed) => panic!(
            "timed out waiting for {what} ({elapsed}): {:?}",
            rx.borrow()
        ),
    }
}

fn on(bluetooth: &Bluetooth) -> bool {
    bluetooth.presence == Presence::Present
        && bluetooth.power == BtPower::On
        && bluetooth.devices.len() == 1
}

#[test]
fn bluez_starting_late_dying_and_returning_is_followed() {
    let bus = PrivateBus::start();
    let scratch = std::env::temp_dir().join(format!("shell-bt-bus-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap_or_else(|e| panic!("{e}"));
    let log = scratch.join("powered.log");
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    rt.block_on(async {
        let env = Env {
            buses: Buses::new(
                BusRoute::Address(bus.address().to_owned()),
                BusRoute::Address(bus.address().to_owned()),
            ),
            ..Env::isolated(&scratch)
        };
        // bardo-shell first: no org.bluez yet.
        let service = spawn::<BluetoothService>(&tokio::runtime::Handle::current(), &env, ());
        let mut rx = service.state;
        // Best-effort absence check: the service says nothing when it finds no BlueZ, so it is
        // given a moment to have said something wrong.
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert_eq!(rx.borrow().presence, Presence::Absent);

        // BlueZ starts late.
        let fake = start_fake(&bus, &log);
        until(&mut rx, "On after a late BlueZ", on).await;

        // BlueZ dies: stale On must not stay.
        stop(fake);
        until(&mut rx, "Absent after BlueZ died", |b| {
            b.presence == Presence::Absent && b.devices.is_empty()
        })
        .await;

        // BlueZ restarts.
        let fake = start_fake(&bus, &log);
        until(&mut rx, "On after BlueZ restarted", on).await;
        stop(fake);
        drop(service.control);
    });
    let _ = std::fs::remove_dir_all(&scratch);
}
