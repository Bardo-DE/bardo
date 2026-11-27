//! The syncd binary on a private bus with scratch HOME and XDG (a `TestEnv`): it serves `Sync1` with no dataset,
//! finds callers through a /proc root only in a `test-proc-root` build, keeps its copy of an
//! account accountd says is gone, and refuses to start over a caller table it cannot read.

use crate::common;

use base_testkit::{PrivateBus, Reaped, TestEnv};
use common::error_name;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use sync_bus::SYNC_BUS;
use sync_bus::SyncProxy;

/// The daemon process, killed by its PID when the guard goes.
struct Daemon {
    child: Reaped,
    stderr: PathBuf,
}

impl Daemon {
    fn stderr(&self) -> String {
        std::fs::read_to_string(&self.stderr).unwrap_or_default()
    }
}

/// A scratch environment on `bus`; its system caller table is absent (so empty: nobody has a
/// fixed role) rather than this computer's `/etc/bardo`.
fn env_on(bus: &PrivateBus) -> TestEnv {
    let env = TestEnv::new("syncd").with_bus(bus);
    let table = env.root().join("system-callers.toml").display().to_string();
    env.with(account_callers::SYSTEM_CALLERS_VAR, &table)
}

fn spawn(env: &TestEnv) -> Daemon {
    let stderr = env.root().join("stderr.log");
    let mut command = env.command(env!("CARGO_BIN_EXE_bardo-syncd"));
    command
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(&stderr).expect("stderr file"));
    Daemon {
        child: Reaped::spawn(&mut command).expect("syncd starts"),
        stderr,
    }
}

async fn serving(bus: &PrivateBus, daemon: &mut Daemon) -> bool {
    let probe = bus.connect().await;
    let dbus = zbus::fdo::DBusProxy::new(&probe).await.expect("proxy");
    base_testkit::eventually_some_async("syncd to own its name or end", || {
        let exited = daemon.child.child_mut().try_wait().ok().flatten().is_some();
        let dbus = dbus.clone();
        async move {
            if dbus
                .name_has_owner(SYNC_BUS.try_into().expect("name"))
                .await
                .unwrap_or(false)
            {
                return Some(true);
            }
            exited.then_some(false)
        }
    })
    .await
}

/// A /proc tree naming this process a Flatpak app.
fn proc_tree(dir: &Path) -> PathBuf {
    let root = dir.join("proc");
    let pid = root.join(std::process::id().to_string());
    std::fs::create_dir_all(&pid).expect("proc dir");
    std::fs::write(
        pid.join("cgroup"),
        "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-flatpak-org.example.Probe-1.scope\n",
    )
    .expect("cgroup");
    // A Flatpak app is known by its sandbox's metadata, not its scope's name.
    std::fs::create_dir_all(pid.join("root")).expect("root");
    std::fs::write(
        pid.join("root/.flatpak-info"),
        "[Application]\nname=org.example.Probe\n",
    )
    .expect("flatpak-info");
    root
}

#[cfg(feature = "test-proc-root")]
#[tokio::test(flavor = "multi_thread")]
async fn a_test_build_reads_callers_from_the_proc_root_and_serves_an_empty_sync1() {
    let bus = PrivateBus::start();
    let env = env_on(&bus);
    let proc = proc_tree(env.root()).display().to_string();
    let env = env.with("BARDO_SYNCD_PROC_ROOT", &proc);
    let mut daemon = spawn(&env);
    assert!(serving(&bus, &mut daemon).await, "{}", daemon.stderr());
    assert!(
        daemon.stderr().contains("reading callers from") && daemon.stderr().contains("proc"),
        "{}",
        daemon.stderr()
    );
    let client = bus.connect().await;
    let sync = SyncProxy::new(&client).await.expect("proxy");
    assert_eq!(
        sync.datasets().await.expect("datasets"),
        [] as [std::string::String; 0]
    );
    let err = sync
        .status("a1/pim")
        .await
        .map(drop)
        .expect_err("no dataset yet");
    assert_eq!(error_name(&err), common::NO_FITTING);
}

#[tokio::test(flavor = "multi_thread")]
async fn accountd_saying_an_account_is_gone_does_not_wipe_syncds_copy() {
    let bus = PrivateBus::start();
    let env = env_on(&bus);
    let journal = env.state().join("bardo/sync/a1");
    std::fs::create_dir_all(&journal).expect("dir");
    std::fs::write(journal.join("x"), "data").expect("file");
    let mut daemon = spawn(&env);
    assert!(serving(&bus, &mut daemon).await, "{}", daemon.stderr());
    // This connection plays accountd and says the account is gone, to everyone.
    let accountd = bus.connect().await;
    accountd
        .request_name(account_wire::names::ACCOUNTS_BUS)
        .await
        .expect("name");
    let path = zbus::zvariant::ObjectPath::try_from(format!(
        "{}/account/a1",
        account_wire::names::ACCOUNTS_PATH
    ))
    .expect("path");
    accountd
        .emit_signal(
            None::<zbus::names::BusName<'_>>,
            account_wire::names::ACCOUNTS_PATH,
            "org.bardo.Accounts1.Manager",
            "AccountRemoved",
            &(path,),
        )
        .await
        .expect("emit");
    // A call syncd answers after the signal was sent has been processed after it too.
    let client = bus.connect().await;
    let _ = SyncProxy::new(&client)
        .await
        .expect("proxy")
        .datasets()
        .await;
    assert!(journal.exists(), "the person removes it in the sync rows");
}

#[cfg(not(feature = "test-proc-root"))]
#[tokio::test(flavor = "multi_thread")]
async fn a_release_build_ignores_the_proc_root_variable() {
    let bus = PrivateBus::start();
    let env = env_on(&bus);
    let proc = proc_tree(env.root()).display().to_string();
    let env = env.with("BARDO_SYNCD_PROC_ROOT", &proc);
    let mut daemon = spawn(&env);
    assert!(serving(&bus, &mut daemon).await, "{}", daemon.stderr());
    assert!(
        !daemon.stderr().contains("reading callers from"),
        "{}",
        daemon.stderr()
    );
    // The real /proc says this process is no app (a test runner's scope): nobody, so refused.
    let client = bus.connect().await;
    let err = SyncProxy::new(&client)
        .await
        .expect("proxy")
        .datasets()
        .await
        .expect_err("the fixture was not read");
    assert_eq!(error_name(&err), common::ACCESS_DENIED);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_caller_table_that_does_not_parse_stops_the_daemon_naming_the_file() {
    let bus = PrivateBus::start();
    let env = env_on(&bus);
    let config = env.config().join("bardo/accounts");
    std::fs::create_dir_all(&config).expect("config dir");
    std::fs::write(
        config.join("callers.toml"),
        "[[caller]]\napp = \"not a name\"\nrole = \"app\"\n",
    )
    .expect("callers");
    let mut daemon = spawn(&env);
    assert!(!serving(&bus, &mut daemon).await);
    assert!(!daemon.child.child_mut().wait().expect("exits").success());
    assert!(
        daemon.stderr().contains("callers.toml"),
        "{}",
        daemon.stderr()
    );
}
