//! `serving` against a private bus: never the session bus.

use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

const BUS_CONF: &str = r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:dir=DIR</listen>
  <auth>EXTERNAL</auth>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>
"#;

/// A dbus-daemon of its own, stopped on drop.
struct PrivateBus {
    daemon: Child,
    address: String,
    _dir: tempfile::TempDir,
}

impl PrivateBus {
    async fn start() -> Self {
        let dir = tempfile::tempdir().expect("scratch dir");
        let conf = dir.path().join("bus.conf");
        std::fs::write(
            &conf,
            BUS_CONF.replace("DIR", &dir.path().display().to_string()),
        )
        .expect("bus config");
        let mut daemon = Command::new("dbus-daemon")
            .arg(format!("--config-file={}", conf.display()))
            .args(["--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .expect("dbus-daemon is needed for these tests");
        let mut lines = BufReader::new(daemon.stdout.take().expect("stdout")).lines();
        let address = tokio::time::timeout(Duration::from_secs(10), lines.next_line())
            .await
            .expect("dbus-daemon printed its address")
            .expect("reading the address")
            .expect("an address line");
        Self {
            daemon,
            address,
            _dir: dir,
        }
    }

    async fn connect(&self) -> zbus::Connection {
        zbus::connection::Builder::address(self.address.as_str())
            .expect("address")
            .method_timeout(Duration::from_secs(5))
            .build()
            .await
            .expect("connecting to the private bus")
    }
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.daemon.start_kill();
    }
}

struct Echo;

#[zbus::interface(name = "org.example.Echo1")]
impl Echo {
    fn echo(&self, text: String) -> String {
        text
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_name_claimed_after_serving_answers_its_first_call() {
    let bus = PrivateBus::start().await;
    for _ in 0..20 {
        let server = bus.connect().await;
        let client = bus.connect().await;
        server
            .object_server()
            .at("/org/example/Echo", Echo)
            .await
            .expect("registered");
        zbus_ready::serving(&server)
            .await
            .expect("the server answers");
        server
            .request_name("org.example.Echo")
            .await
            .expect("named");
        let reply = client
            .call_method(
                Some("org.example.Echo"),
                "/org/example/Echo",
                Some("org.example.Echo1"),
                "Echo",
                &("hi",),
            )
            .await
            .expect("the first call is answered");
        let text: String = reply.body().deserialize().expect("a string");
        assert_eq!(text, "hi");
        server
            .release_name("org.example.Echo")
            .await
            .expect("released");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_connection_with_no_objects_still_comes_ready() {
    let bus = PrivateBus::start().await;
    let connection = bus.connect().await;
    zbus_ready::serving(&connection)
        .await
        .expect("the root node answers Peer");
}
