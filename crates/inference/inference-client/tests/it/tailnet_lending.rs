//! `TailnetLending`: the Settings switch's file work, against a scratch systemd user directory and
//! a manager that records its calls: the exact bytes and mode written, the order of the calls,
//! the states a file can be in and what each failure leaves behind. `SessionUnits` is tested on a
//! private bus against a scripted `org.freedesktop.systemd1` manager. Nothing here touches the
//! real systemd, `~/.config` or `/etc`.
#![cfg(feature = "lending")]

use inference_client::{
    LendingConfig, LendingError, LendingState, TailnetLending, UnitFailure, UnitManager,
};
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// What the shipped file holds in these tests.
const SHIPPED: &str = "[Service]\nIPAddressAllow=100.64.0.0/10 fd7a:115c:a1e0::/48\n";

/// Where the manager is told to fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fail {
    Nowhere,
    Reload,
    Restart,
}

/// A manager that records its calls in order and fails where told.
#[derive(Debug, Clone)]
struct FakeUnits {
    calls: Arc<Mutex<Vec<String>>>,
    fail: Fail,
}

impl FakeUnits {
    fn new(fail: Fail) -> Self {
        Self {
            calls: Arc::default(),
            fail,
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("calls").clone()
    }
}

impl UnitManager for FakeUnits {
    async fn reload(&self) -> Result<(), UnitFailure> {
        self.calls.lock().expect("calls").push("reload".to_owned());
        match self.fail {
            Fail::Reload => Err(UnitFailure("scripted".to_owned())),
            _ => Ok(()),
        }
    }

    async fn restart(&self, unit: &'static str) -> Result<(), UnitFailure> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("restart {unit}"));
        match self.fail {
            Fail::Restart => Err(UnitFailure("scripted".to_owned())),
            _ => Ok(()),
        }
    }
}

/// A scratch directory with the shipped file in it.
struct Fixture {
    root: base_testkit::Scratch,
}

impl Fixture {
    fn new() -> Self {
        let root = base_testkit::Scratch::new("lending");
        fs::create_dir_all(root.join("share")).expect("scratch");
        fs::write(root.join("share/inferd-tailnet.conf"), SHIPPED).expect("shipped");
        Self { root }
    }

    fn config(&self) -> LendingConfig {
        LendingConfig::new(self.root.join("share/inferd-tailnet.conf"), self.user_dir())
    }

    fn user_dir(&self) -> PathBuf {
        self.root.join("config/systemd/user")
    }

    fn drop_in(&self) -> PathBuf {
        self.user_dir().join("inferd.service.d/tailnet.conf")
    }

    fn lending(&self, fail: Fail) -> (TailnetLending<FakeUnits>, FakeUnits) {
        let units = FakeUnits::new(fail);
        (TailnetLending::new(self.config(), units.clone()), units)
    }

    fn put(&self, text: &str) {
        fs::create_dir_all(self.drop_in().parent().expect("dir")).expect("dir");
        fs::write(self.drop_in(), text).expect("put");
    }
}

fn mode(path: &Path) -> u32 {
    fs::metadata(path).expect("meta").permissions().mode() & 0o777
}

const RELOAD_RESTART: [&str; 2] = ["reload", "restart inferd.service"];

#[tokio::test]
async fn enable_writes_the_shipped_bytes_readable_by_all_then_reloads_and_restarts_in_that_order() {
    let s = Fixture::new();
    let (lending, units) = s.lending(Fail::Nowhere);
    assert_eq!(lending.state(), LendingState::NotInstalled);

    lending.enable().await.expect("enable");

    assert_eq!(fs::read_to_string(s.drop_in()).expect("file"), SHIPPED);
    assert_eq!(mode(&s.drop_in()), 0o644);
    assert_eq!(units.calls(), RELOAD_RESTART);
    assert_eq!(lending.state(), LendingState::Installed);
    // Nothing is left staged beside it.
    let beside: Vec<_> = fs::read_dir(s.drop_in().parent().expect("dir"))
        .expect("dir")
        .map(|e| e.expect("entry").file_name())
        .collect();
    assert_eq!(beside, [OsString::from("tailnet.conf")]);
}

#[tokio::test]
async fn enable_gives_0644_whatever_the_umask_made_of_a_new_file() {
    // The staged file is made with the process's umask; the mode is then set outright. A
    // restrictive file already there is replaced, not kept.
    let s = Fixture::new();
    s.put("# mine\n");
    fs::set_permissions(s.drop_in(), fs::Permissions::from_mode(0o600)).expect("chmod");
    let (lending, _) = s.lending(Fail::Nowhere);
    lending.enable().await.expect("enable");
    assert_eq!(mode(&s.drop_in()), 0o644);
}

#[tokio::test]
async fn a_file_that_is_not_the_shipped_one_reads_as_differs_and_enable_replaces_it() {
    let s = Fixture::new();
    s.put("[Service]\nIPAddressAllow=any\n");
    let (lending, units) = s.lending(Fail::Nowhere);
    assert_eq!(lending.state(), LendingState::Differs);

    lending.enable().await.expect("enable");

    assert_eq!(fs::read_to_string(s.drop_in()).expect("file"), SHIPPED);
    assert_eq!(lending.state(), LendingState::Installed);
    assert_eq!(units.calls(), RELOAD_RESTART);
}

#[tokio::test]
async fn a_shipped_file_that_changed_since_the_install_makes_the_old_copy_differ() {
    let s = Fixture::new();
    let (lending, _) = s.lending(Fail::Nowhere);
    lending.enable().await.expect("enable");
    fs::write(
        s.config().shipped(),
        "[Service]\nIPAddressAllow=100.64.0.0/10\n",
    )
    .expect("newer");
    assert_eq!(lending.state(), LendingState::Differs);
}

#[tokio::test]
async fn disable_takes_the_drop_in_and_its_empty_folder_away_then_reloads_and_restarts() {
    let s = Fixture::new();
    let (lending, units) = s.lending(Fail::Nowhere);
    lending.enable().await.expect("enable");

    lending.disable().await.expect("disable");

    assert!(!s.drop_in().exists());
    assert!(!s.drop_in().parent().expect("dir").exists());
    assert_eq!(lending.state(), LendingState::NotInstalled);
    assert_eq!(
        units.calls(),
        [
            "reload",
            "restart inferd.service",
            "reload",
            "restart inferd.service"
        ]
    );
}

#[tokio::test]
async fn disable_with_nothing_installed_is_fine_and_does_it_again() {
    let s = Fixture::new();
    let (lending, units) = s.lending(Fail::Nowhere);
    lending.disable().await.expect("first");
    lending.disable().await.expect("second");
    assert_eq!(lending.state(), LendingState::NotInstalled);
    assert_eq!(
        units.calls(),
        [
            "reload",
            "restart inferd.service",
            "reload",
            "restart inferd.service"
        ]
    );
}

#[tokio::test]
async fn disable_leaves_the_persons_other_files_in_the_folder() {
    let s = Fixture::new();
    let (lending, _) = s.lending(Fail::Nowhere);
    lending.enable().await.expect("enable");
    let other = s.drop_in().with_file_name("mine.conf");
    fs::write(&other, "# mine\n").expect("other");

    lending.disable().await.expect("disable");

    assert!(!s.drop_in().exists());
    assert_eq!(fs::read_to_string(other).expect("kept"), "# mine\n");
}

#[tokio::test]
async fn a_missing_shipped_file_changes_nothing_and_asks_nothing_of_the_manager() {
    let s = Fixture::new();
    let config = LendingConfig::new(s.root.join("share/none.conf"), s.user_dir());
    let units = FakeUnits::new(Fail::Nowhere);
    let lending = TailnetLending::new(config, units.clone());

    let error = lending.enable().await.expect_err("missing");

    assert!(
        matches!(error, LendingError::ShippedFileMissing),
        "{error:?}"
    );
    assert!(!s.user_dir().exists());
    assert!(units.calls().is_empty());
    // And a file that is there cannot be the shipped one.
    s.put(SHIPPED);
    assert_eq!(lending.state(), LendingState::Differs);
}

/// What enabling or disabling does when the folder or the manager fails: the error says which
/// step it was, the manager was asked as far as that step, the file is as far as it got, and a
/// second try does it all again.
#[tokio::test]
async fn a_failure_names_its_step_and_a_second_try_does_it_all_again() {
    struct Row {
        what: &'static str,
        fail: Fail,
        /// `inferd.service.d` is a file, so the drop-in cannot go in it.
        blocked: bool,
        /// The shipped file is already installed.
        installed: bool,
        disable: bool,
        error: fn(&LendingError) -> bool,
        calls: &'static [&'static str],
        /// Whether the drop-in is there afterwards.
        kept: Option<bool>,
    }
    let rows = [
        Row {
            what: "a folder that cannot be made",
            fail: Fail::Nowhere,
            blocked: true,
            installed: false,
            disable: false,
            error: |e| matches!(e, LendingError::CouldNotWrite(_)),
            calls: &[],
            kept: None,
        },
        Row {
            what: "a reload that fails",
            fail: Fail::Reload,
            blocked: false,
            installed: false,
            disable: false,
            error: |e| matches!(e, LendingError::ReloadFailed(_)),
            calls: &["reload"],
            kept: Some(true),
        },
        Row {
            what: "a restart that fails",
            fail: Fail::Restart,
            blocked: false,
            installed: false,
            disable: false,
            error: |e| matches!(e, LendingError::RestartFailed(_)),
            calls: &RELOAD_RESTART,
            kept: Some(true),
        },
        Row {
            what: "a disable whose restart fails",
            fail: Fail::Restart,
            blocked: false,
            installed: true,
            disable: true,
            error: |e| matches!(e, LendingError::RestartFailed(_)),
            calls: &RELOAD_RESTART,
            kept: Some(false),
        },
    ];
    for row in rows {
        let s = Fixture::new();
        if row.blocked {
            fs::create_dir_all(s.user_dir()).expect("dir");
            fs::write(s.user_dir().join("inferd.service.d"), "in the way").expect("block");
        }
        if row.installed {
            s.put(SHIPPED);
        }
        let (lending, units) = s.lending(row.fail);
        for time in 1..=2 {
            let result = if row.disable {
                lending.disable().await
            } else {
                lending.enable().await
            };
            let error = result.expect_err(row.what);
            assert!((row.error)(&error), "{}: {error:?}", row.what);
            assert_eq!(
                units.calls(),
                row.calls.repeat(time),
                "{} (try {time})",
                row.what
            );
            if let Some(kept) = row.kept {
                assert_eq!(s.drop_in().exists(), kept, "{}", row.what);
            }
        }
    }
}

/// The environment's answers, as a table gives them.
fn lookup<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
    move |name| {
        vars.iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| OsString::from(*value))
    }
}

#[test]
fn the_environment_names_the_persons_folder_and_where_the_shipped_file_is() {
    let s = Fixture::new();
    let data = s.root.to_str().expect("utf-8");
    fs::create_dir_all(s.root.join("porter")).expect("dir");
    fs::write(s.root.join("porter/inferd-tailnet.conf"), SHIPPED).expect("file");

    // (variables, the drop-in's folder, the shipped file)
    type Row<'a> = (Vec<(&'a str, &'a str)>, Option<&'a str>, Option<PathBuf>);
    let rows: [Row; 6] = [
        (
            vec![("HOME", "/home/ada"), ("XDG_DATA_DIRS", data)],
            Some("/home/ada/.config/systemd/user"),
            Some(s.root.join("porter/inferd-tailnet.conf")),
        ),
        (
            vec![
                ("HOME", "/home/ada"),
                ("XDG_CONFIG_HOME", "/home/ada/cfg"),
                ("XDG_DATA_DIRS", data),
            ],
            Some("/home/ada/cfg/systemd/user"),
            Some(s.root.join("porter/inferd-tailnet.conf")),
        ),
        // A relative XDG_CONFIG_HOME is ignored, as the XDG rules say.
        (
            vec![("HOME", "/home/ada"), ("XDG_CONFIG_HOME", "cfg")],
            Some("/home/ada/.config/systemd/user"),
            None,
        ),
        // No file is found: the first place it would be is named, so `enable` says it is missing.
        (
            vec![("HOME", "/home/ada"), ("XDG_DATA_DIRS", "/nowhere/share")],
            Some("/home/ada/.config/systemd/user"),
            Some(PathBuf::from(
                "/home/ada/.local/share/porter/inferd-tailnet.conf",
            )),
        ),
        // No home folder and no XDG_CONFIG_HOME: nowhere to put it.
        (vec![], None, None),
        (vec![("HOME", "relative")], None, None),
    ];
    for (vars, folder, shipped) in rows {
        let found = LendingConfig::from_lookup(lookup(&vars));
        match folder {
            None => assert!(found.is_none(), "{vars:?}"),
            Some(folder) => {
                let found = found.unwrap_or_else(|| panic!("{vars:?}"));
                assert_eq!(
                    found.drop_in(),
                    PathBuf::from(folder).join("inferd.service.d/tailnet.conf"),
                    "{vars:?}"
                );
                if let Some(shipped) = shipped {
                    assert_eq!(found.shipped(), shipped, "{vars:?}");
                }
            }
        }
    }
}

#[test]
fn a_prefix_names_the_shipped_file_where_install_sh_puts_it() {
    let config = LendingConfig::under_prefix(Path::new("/usr"), PathBuf::from("/x"));
    assert_eq!(
        config.shipped(),
        Path::new("/usr/share/porter/inferd-tailnet.conf")
    );
}

#[cfg(feature = "dbus")]
mod session_bus {
    use super::*;
    use crate::common::bus::PrivateBus;
    use inference_client::SessionUnits;
    use zbus::fdo;
    use zbus::object_server::SignalEmitter;
    use zbus::zvariant::{ObjectPath, OwnedObjectPath};

    /// A scripted manager whose restart job ends `failed`.
    struct Manager {
        calls: Arc<Mutex<Vec<String>>>,
    }

    #[zbus::interface(name = "org.freedesktop.systemd1.Manager")]
    impl Manager {
        async fn reload(&self) {
            self.calls.lock().expect("calls").push("Reload".to_owned());
        }

        async fn restart_unit(
            &self,
            name: &str,
            mode: &str,
            #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        ) -> fdo::Result<OwnedObjectPath> {
            self.calls
                .lock()
                .expect("calls")
                .push(format!("RestartUnit {name} {mode}"));
            let other = ObjectPath::try_from("/org/freedesktop/systemd1/job/1").expect("path");
            let ours = ObjectPath::try_from("/org/freedesktop/systemd1/job/2").expect("path");
            // Another job finishes first: it is not ours and must not end the wait.
            Self::job_removed(&emitter, 1, other, "other.service", "failed")
                .await
                .map_err(|e| fdo::Error::Failed(e.to_string()))?;
            Self::job_removed(&emitter, 2, ours.clone(), name, "failed")
                .await
                .map_err(|e| fdo::Error::Failed(e.to_string()))?;
            Ok(ours.into())
        }

        #[zbus(signal)]
        async fn job_removed(
            emitter: &SignalEmitter<'_>,
            id: u32,
            job: ObjectPath<'_>,
            unit: &str,
            result: &str,
        ) -> zbus::Result<()>;
    }

    async fn run() -> (Result<(), LendingError>, Vec<String>, Fixture) {
        let bus = PrivateBus::start();
        let systemd = bus.connect().await;
        let calls = Arc::new(Mutex::new(Vec::new()));
        systemd
            .object_server()
            .at(
                "/org/freedesktop/systemd1",
                Manager {
                    calls: Arc::clone(&calls),
                },
            )
            .await
            .expect("manager");
        zbus_ready::serving(&systemd)
            .await
            .expect("the object server answers");
        systemd
            .request_name("org.freedesktop.systemd1")
            .await
            .expect("name");
        let s = Fixture::new();
        let lending = TailnetLending::new(s.config(), SessionUnits::new(bus.connect().await));
        let result = lending.enable().await;
        let calls = calls.lock().expect("calls").clone();
        (result, calls, s)
    }

    #[tokio::test]
    async fn a_restart_job_that_ends_failed_is_a_failed_restart() {
        let (result, calls, _) = run().await;
        let error = result.expect_err("failed job");
        assert!(matches!(error, LendingError::RestartFailed(_)), "{error:?}");
        assert_eq!(calls, ["Reload", "RestartUnit inferd.service replace"]);
    }

    #[tokio::test]
    async fn no_manager_on_the_bus_is_a_failed_reload() {
        let bus = PrivateBus::start();
        let s = Fixture::new();
        let lending = TailnetLending::new(s.config(), SessionUnits::new(bus.connect().await));
        let error = lending.enable().await.expect_err("no manager");
        assert!(matches!(error, LendingError::ReloadFailed(_)), "{error:?}");
    }
}
