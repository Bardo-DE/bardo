//! The real `readerd` binary on a private bus: it claims `org.bardo.Reader1`, refuses a caller that
//! is not intentd's unit, claims the name once and frees it when it stops. Nothing of the real
//! session is named: the bus, HOME and every XDG directory are scratch. The binary reads the
//! system's `/proc`, which no test may move, so who it does answer (intentd's unit, as the
//! callers file names it) is tested in process, over a fake proc root (`callers.rs`).

use crate::support::bus::PrivateBus;
use action_core::{ReaderAsk, ReaderTask, ValueSchema};
use action_dbus::{Details, ReaderProxy};
use action_vocab::Handle;
use base_testkit::GENEROUS;
use futures_util::StreamExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use zbus::fdo::DBusProxy;

/// A child that is killed when the test ends, however it ends.
struct Daemon(Child);

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn readerd(dir: &Path, bus: &str) -> Daemon {
    let home = dir.display().to_string();
    Daemon(
        Command::new(env!("CARGO_BIN_EXE_bardo-readerd"))
            .env_clear()
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", dir.join("config"))
            .env("XDG_CONFIG_DIRS", dir.join("none"))
            .env("DBUS_SESSION_BUS_ADDRESS", bus)
            .stdin(Stdio::null())
            .spawn()
            .expect("readerd starts"),
    )
}

async fn has_owner(connection: &zbus::Connection, name: &str) -> bool {
    let dbus = DBusProxy::new(connection).await.expect("dbus");
    let name = zbus::names::BusName::try_from(name).expect("name");
    dbus.name_has_owner(name).await.unwrap_or(false)
}

/// Waits until `name` has an owner (`owned`) or has none, on the bus's own announcement of a
/// change: subscribed before the first look, so a change between the look and the wait is kept.
async fn until_owner(connection: &zbus::Connection, name: &str, owned: bool) {
    let dbus = DBusProxy::new(connection).await.expect("dbus");
    let mut changes = dbus
        .receive_name_owner_changed_with_args(&[(0, name)])
        .await
        .expect("subscribe to the owner changes");
    tokio::time::timeout(GENEROUS, async {
        while has_owner(connection, name).await != owned {
            changes.next().await.expect("the bus ended the signals");
        }
    })
    .await
    .unwrap_or_else(|_| panic!("{name} never had owned = {owned}"));
}

async fn until_served(connection: &zbus::Connection) {
    until_owner(connection, "org.bardo.Reader1", true).await;
}

fn ask() -> String {
    serde_json::to_string(&ReaderAsk {
        inputs: vec![Handle(1)],
        want: ValueSchema::Integer { min: 0, max: 9 },
        task: ReaderTask::Extract,
    })
    .expect("json")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_binary_refuses_a_stranger_claims_the_name_once_and_frees_it_when_it_stops() {
    let dir = tempfile::tempdir().expect("scratch");
    let bus = PrivateBus::start(dir.path());
    let me = bus.connect().await;
    let mut daemon = readerd(dir.path(), bus.address());
    until_served(&me).await;

    // A process that is not intentd's unit is refused before the ask is read, whatever the
    // system's `/proc` says this test process is: only bardo-actiond.service's main process passes.
    let stranger = ReaderProxy::new(&me).await.expect("proxy");
    let refused = stranger
        .extract("s-1", &ask(), &Details::new())
        .await
        .expect_err("a stranger is not answered");
    assert!(
        matches!(&refused, zbus::Error::MethodError(name, _, _)
            if name.as_str() == "org.freedesktop.DBus.Error.AccessDenied"),
        "{refused}"
    );

    // A second readerd finds the name taken and stops, leaving the first serving.
    let mut second = readerd(dir.path(), bus.address());
    let status = tokio::task::spawn_blocking(move || second.0.wait())
        .await
        .expect("joined")
        .expect("waited");
    assert!(
        !status.success(),
        "a second readerd is not served: {status:?}"
    );
    assert!(has_owner(&me, "org.bardo.Reader1").await);

    // Stopping it frees the name.
    daemon.0.kill().expect("kill");
    daemon.0.wait().expect("wait");
    until_owner(&me, "org.bardo.Reader1", false).await;
}
