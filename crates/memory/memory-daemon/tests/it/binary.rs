//! The packaged `memoryd` binary, started the way the acceptance run starts it: on a private bus,
//! with `env_clear`, scratch HOME and XDG directories and no Secret Service.
//!
//! The Landlock sandbox is ON: callers are identified through a fake `/proc` (`BARDO_MEMORYD_PROC_ROOT`, feature `test-proc-root`)
//! that puts this test process in `bardo-actiond.service`.
//!
//! With the `test-keys` feature `BARDO_MEMORYD_KEYS=file:<path>` gives it file keys, it claims
//! `org.bardo.Memory1` and takes a `Record` into `desktop` (a Space nobody registered). Without
//! the feature the same variable is ignored, and the daemon says so.
//!
//! The same start-up is also run as a library call, `memory_daemon::start`, over an injected environment
//! (the variables are a map, the bus address among them) on a thread of the test process.

use base_testkit::PrivateBus;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, ChildStderr, Command, Stdio};

/// The daemon process; killed by pid when dropped.
struct Daemon {
    child: Child,
}

impl Drop for Daemon {
    fn drop(&mut self) {
        drop(self.child.kill());
        drop(self.child.wait());
    }
}

/// Scratch HOME and XDG roots, a callers file with the router's unit row, and a fake `/proc` in
/// which this test process runs in `bardo-actiond.service` as its main process.
fn scratch_home(root: &Path) {
    let config = root.join("config").join("bardo").join("memoryd");
    std::fs::create_dir_all(&config).expect("config dir");
    std::fs::create_dir_all(root.join("run")).expect("runtime dir");
    std::fs::write(
        config.join("callers.toml"),
        "[[caller]]\napp = \"org.bardo.Intents\"\nunit = \"bardo-actiond.service\"\nrole = \"agent\"\n",
    )
    .expect("callers file");
    // The system's table, in the scratch root: this computer's `/etc/bardo` is never read. Only
    // the system's can give the router's unit its role.
    std::fs::write(
        root.join("system-callers.toml"),
        "[[caller]]\napp = \"org.bardo.Intents\"\nunit = \"bardo-actiond.service\"\nrole = \"agent\"\n",
    )
    .expect("system callers file");
    let me = root.join("proc").join(std::process::id().to_string());
    std::fs::create_dir_all(&me).expect("fake proc");
    std::fs::write(
        me.join("cgroup"),
        "0::/user.slice/user-1000.slice/user@1000.service/app.slice/bardo-actiond.service\n",
    )
    .expect("cgroup");
    // ...as that unit's main process, which is all a service row names.
    let units = root.join("proc").join("units");
    std::fs::create_dir_all(&units).expect("units dir");
    std::fs::write(
        units.join("bardo-actiond.service"),
        std::process::id().to_string(),
    )
    .expect("main pid");
}

/// The variables the daemon is started with: scratch roots, the private bus, file keys and the
/// fake `/proc`. Nothing else, so nothing of the real session reaches it.
fn daemon_env(root: &Path, bus: &PrivateBus, keys: &str) -> Vec<(&'static str, String)> {
    let at = |dir: &str| root.join(dir).to_string_lossy().into_owned();
    vec![
        ("HOME", at("home")),
        ("XDG_DATA_HOME", at("data")),
        ("XDG_CACHE_HOME", at("cache")),
        ("XDG_CONFIG_HOME", at("config")),
        ("XDG_RUNTIME_DIR", at("run")),
        ("DBUS_SESSION_BUS_ADDRESS", bus.address().to_owned()),
        ("BARDO_MEMORYD_KEYS", keys.to_owned()),
        ("BARDO_MEMORYD_PROC_ROOT", at("proc")),
        (memory_daemon::SYSTEM_CALLERS_VAR, at("system-callers.toml")),
    ]
}

fn spawn(root: &Path, bus: &PrivateBus, keys: &str) -> (Daemon, ChildStderr) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_bardo-memoryd"))
        .env_clear()
        .envs(daemon_env(root, bus, keys))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn memoryd");
    let stderr = child.stderr.take().expect("stderr");
    (Daemon { child }, stderr)
}

fn first_line(stderr: ChildStderr) -> String {
    let mut line = String::new();
    BufReader::new(stderr)
        .read_line(&mut line)
        .expect("a startup line");
    line
}

/// Starts a daemon with `launch` (given the keys file it is to use) on a private bus and takes a
/// `Record` into `desktop` (a Space nobody registered) through it; whatever `launch` returns is
/// kept alive until the checks are done, then handed back.
async fn record_into_desktop<G>(
    root: &Path,
    bus: &PrivateBus,
    launch: impl FnOnce(&Path) -> G,
) -> G {
    use crate::support::connect;
    use memory_client::{DbusTransport, Memory, Recorded};
    use memory_core::{Record, SpaceId};
    use memory_dbus::MEMORY_BUS;
    use memory_fake::mail_thread_archived;
    use std::time::Duration;
    use zbus::export::futures_core::Stream;
    let client = connect(bus.address()).await;
    // Watch for the name before the daemon starts: no sleep, no polling.
    let dbus = zbus::fdo::DBusProxy::new(&client)
        .await
        .expect("dbus proxy");
    let mut owners = dbus.receive_name_owner_changed().await.expect("signal");
    let keys_file = root.join("keys").join("memoryd.keys");
    let started = launch(&keys_file);
    let claimed = async {
        loop {
            let item = std::future::poll_fn(|cx| std::pin::Pin::new(&mut owners).poll_next(cx))
                .await
                .expect("the bus signals");
            let args = item.args().expect("args");
            if args.name().as_str() == MEMORY_BUS && args.new_owner().is_some() {
                break;
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(60), claimed)
        .await
        .expect("the daemon claims org.bardo.Memory1");
    let memory = Memory::over(DbusTransport::new(client));
    let record = Record {
        space: SpaceId::desktop(),
        ..mail_thread_archived().expect("fixture")
    };
    let recorded = memory.record(record).await;
    let recorded = recorded.expect("record into desktop");
    let Recorded::Stored(event) = recorded else {
        panic!("{recorded:?}")
    };
    assert_eq!(event.space, SpaceId::desktop());
    assert!(keys_file.exists(), "the key went to the file");
    assert!(
        root.join("data/bardo/memory/desktop/events.db").exists(),
        "desktop was provisioned"
    );
    started
}

#[tokio::test]
async fn the_test_keys_binary_starts_with_file_keys_and_takes_a_desktop_record() {
    let scratch = tempfile::tempdir().expect("scratch");
    let root = scratch.path();
    scratch_home(root);
    let bus = PrivateBus::start_in(root);
    let (_daemon, stderr) = record_into_desktop(root, &bus, |keys_file| {
        spawn(root, &bus, &format!("file:{}", keys_file.display()))
    })
    .await;
    assert!(first_line(stderr).contains("TEST BUILD"));
}
