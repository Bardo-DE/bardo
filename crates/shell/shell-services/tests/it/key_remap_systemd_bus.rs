//! The key remap daemon service's systemd path against a stand-in `org.freedesktop.systemd1` on a private
//! bus playing the system bus, never the real one. The stand-in's `Manager` answers `GetUnit`
//! with a unit object whose `ActiveState` the test sets, or with `NoSuchUnit`, and records the
//! `ResetFailedUnit` and `RestartUnit` calls. The unit waits are shortened through
//! `Env::key_remap_unit_wait`. The tests need a `dbus-daemon` and skip, saying so, without one.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use base_testkit::{PrivateBus, Scratch};
use shell_model::apps::AppIndex;
use shell_model::key_remap::KeyRemapService;
use shell_model::key_remap::{Attention, KeyRemapCommand, KeyRemapState, notice_text};
use shell_model::notifications::{NotesState, NotificationsCommand};
use shell_model::service::Control;
use shell_model::windows::WindowModel;
use shell_services::env::{BusRoute, Buses, Env, UnitWait};
use shell_services::key_remap::KeyRemapLinks;
use shell_services::service::spawn;
use tokio::sync::{mpsc, watch};
use zbus::interface;
use zbus::zvariant::OwnedObjectPath;

/// How long a thing that must not happen is given to happen: a best-effort absence check, as
/// nothing announces a call or a notice that never comes.
const SHORT: Duration = Duration::from_millis(400);
const LONG: Duration = Duration::from_secs(5);

const UNIT_PATH: &str = "/org/freedesktop/systemd1/unit/bardo_2dkey_2dremap_2eservice";

/// What the stand-in systemd holds and heard.
#[derive(Debug, Default)]
struct Systemd {
    /// Whether the manager has the unit loaded.
    loaded: bool,
    /// The unit's `ActiveState`.
    active_state: String,
    /// The calls heard, in order, as `Method(args)`.
    calls: Vec<String>,
}

type Shared = Arc<Mutex<Systemd>>;

fn lock(shared: &Shared) -> std::sync::MutexGuard<'_, Systemd> {
    shared
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.freedesktop.systemd1")]
enum SystemdError {
    #[zbus(error)]
    ZBus(zbus::Error),
    NoSuchUnit(String),
}

struct Manager(Shared);

#[interface(name = "org.freedesktop.systemd1.Manager")]
impl Manager {
    fn get_unit(&self, name: &str) -> Result<OwnedObjectPath, SystemdError> {
        let mut held = lock(&self.0);
        held.calls.push(format!("GetUnit({name})"));
        if held.loaded && name == "bardo-key-remap.service" {
            Ok(OwnedObjectPath::try_from(UNIT_PATH).map_err(|e| SystemdError::ZBus(e.into()))?)
        } else {
            Err(SystemdError::NoSuchUnit(format!("Unit {name} not loaded.")))
        }
    }

    fn reset_failed_unit(&self, name: &str) {
        lock(&self.0).calls.push(format!("ResetFailedUnit({name})"));
    }

    fn restart_unit(&self, name: &str, mode: &str) -> OwnedObjectPath {
        lock(&self.0)
            .calls
            .push(format!("RestartUnit({name}, {mode})"));
        OwnedObjectPath::try_from("/org/freedesktop/systemd1/job/1")
            .unwrap_or_else(|e| panic!("{e}"))
    }
}

struct Unit(Shared);

#[interface(name = "org.freedesktop.systemd1.Unit")]
impl Unit {
    #[zbus(property)]
    fn active_state(&self) -> String {
        lock(&self.0).active_state.clone()
    }
}

/// The stand-in on `bus`, serving as `org.freedesktop.systemd1`; dropped with the connection.
async fn serve(bus: &PrivateBus, loaded: bool, active_state: &str) -> (zbus::Connection, Shared) {
    let shared: Shared = Arc::new(Mutex::new(Systemd {
        loaded,
        active_state: active_state.to_owned(),
        calls: Vec::new(),
    }));
    let connection = zbus::connection::Builder::address(bus.address())
        .unwrap_or_else(|e| panic!("{e}"))
        .name("org.freedesktop.systemd1")
        .unwrap_or_else(|e| panic!("{e}"))
        .serve_at("/org/freedesktop/systemd1", Manager(Arc::clone(&shared)))
        .unwrap_or_else(|e| panic!("{e}"))
        .serve_at(UNIT_PATH, Unit(Arc::clone(&shared)))
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("the stand-in systemd: {e}"));
    (connection, shared)
}

/// The service with no daemon behind its socket, and the other services' ends.
struct Rig {
    state: watch::Receiver<KeyRemapState>,
    control: Control<KeyRemapCommand>,
    notices: mpsc::UnboundedReceiver<NotificationsCommand>,
    /// Held so the other services' channels stay open.
    _held: (
        watch::Sender<WindowModel>,
        watch::Sender<AppIndex>,
        watch::Sender<NotesState>,
        Scratch,
    ),
}

/// The service with `system` as its system bus and a socket path nobody listens on.
fn rig(system: BusRoute) -> Rig {
    let scratch = Scratch::new("key-remap-systemd");
    let env = Env {
        key_remap_socket: Some(scratch.path().join("key-remap.sock")),
        buses: Buses::new(system, BusRoute::Absent),
        key_remap_unit_wait: UnitWait {
            grace: Duration::from_millis(100),
            recheck: Duration::from_millis(100),
        },
        ..Env::isolated(scratch.path())
    };
    let (wl, wl_rx) = watch::channel(WindowModel::default());
    let (apps, apps_rx) = watch::channel(AppIndex::default());
    let (notes, notes_rx) = watch::channel(NotesState::default());
    let (notifications, notices) = Control::channel();
    let links = KeyRemapLinks {
        windows: wl_rx,
        apps: apps_rx,
        notifications,
        notes: notes_rx,
    };
    let handle = spawn::<KeyRemapService>(&tokio::runtime::Handle::current(), &env, links);
    Rig {
        state: handle.state,
        control: handle.control,
        notices,
        _held: (wl, apps, notes, scratch),
    }
}

/// A private bus and the stand-in on it.
async fn with_systemd(loaded: bool, active_state: &str) -> (PrivateBus, zbus::Connection, Shared) {
    let bus = PrivateBus::start();
    let (connection, shared) = serve(&bus, loaded, active_state).await;
    (bus, connection, shared)
}

async fn until(state: &mut watch::Receiver<KeyRemapState>, want: KeyRemapState) {
    let reached = tokio::time::timeout(LONG, state.wait_for(|s| *s == want))
        .await
        .is_ok_and(|r| r.is_ok());
    assert!(
        reached,
        "gave up waiting for {want:?} at {:?}",
        state.borrow()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_unit_is_failed_and_the_person_is_told() {
    let (bus, _systemd, _shared) = with_systemd(true, "failed").await;
    let mut rig = rig(BusRoute::Address(bus.address().to_owned()));
    until(&mut rig.state, KeyRemapState::Failed).await;
    let posted = tokio::time::timeout(LONG, rig.notices.recv())
        .await
        .unwrap_or_else(|_| panic!("no notice"))
        .unwrap_or_else(|| panic!("the notifications control closed"));
    let NotificationsCommand::Post(posting) = posted else {
        panic!("{posted:?}");
    };
    assert_eq!(posting.summary, notice_text(&Attention::Failed).summary);
    assert_eq!(posting.body.text(), notice_text(&Attention::Failed).body);
    assert_eq!(posting.replaces, None);
    assert_eq!(posting.actions, [] as [shell_model::notifications::Action; 0]);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_unit_is_not_asked_about_within_the_grace_period() {
    let (bus, _systemd, shared) = with_systemd(true, "failed").await;
    let scratch = Scratch::new("key-remap-grace");
    let env = Env {
        key_remap_socket: Some(scratch.path().join("key-remap.sock")),
        buses: Buses::new(
            BusRoute::Address(bus.address().to_owned()),
            BusRoute::Absent,
        ),
        key_remap_unit_wait: UnitWait {
            grace: Duration::from_secs(60),
            recheck: Duration::from_secs(60),
        },
        ..Env::isolated(scratch.path())
    };
    let (_wl, windows) = watch::channel(WindowModel::default());
    let (_apps, apps) = watch::channel(AppIndex::default());
    let (_notes, notes) = watch::channel(NotesState::default());
    let (notifications, mut notices) = Control::channel();
    let handle = spawn::<KeyRemapService>(
        &tokio::runtime::Handle::current(),
        &env,
        KeyRemapLinks {
            windows,
            apps,
            notifications,
            notes,
        },
    );
    tokio::time::sleep(SHORT).await;
    assert_eq!(*handle.state.borrow(), KeyRemapState::Unknown);
    assert_eq!(lock(&shared).calls, [] as [std::string::String; 0]);
    notices.try_recv().unwrap_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_unit_the_manager_has_not_loaded_is_not_installed_and_silent() {
    let (bus, _systemd, shared) = with_systemd(false, "inactive").await;
    let mut rig = rig(BusRoute::Address(bus.address().to_owned()));
    until(&mut rig.state, KeyRemapState::NotInstalled).await;
    tokio::time::sleep(SHORT).await;
    assert_eq!(*rig.state.borrow(), KeyRemapState::NotInstalled);
    assert!(rig.notices.try_recv().is_err(), "nothing is posted");
    let calls = lock(&shared).calls.clone();
    assert_eq!(
        calls,
        ["GetUnit(bardo-key-remap.service)"],
        "a machine with no key_remap is asked once, then left alone"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_activating_unit_is_restarting_and_silent() {
    let (bus, _systemd, _shared) = with_systemd(true, "activating").await;
    let mut rig = rig(BusRoute::Address(bus.address().to_owned()));
    until(&mut rig.state, KeyRemapState::Restarting).await;
    tokio::time::sleep(SHORT).await;
    assert_eq!(*rig.state.borrow(), KeyRemapState::Restarting);
    assert!(rig.notices.try_recv().is_err(), "nothing is posted");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_unit_is_read_again_and_a_later_failure_is_seen() {
    let (bus, _systemd, shared) = with_systemd(true, "activating").await;
    let mut rig = rig(BusRoute::Address(bus.address().to_owned()));
    until(&mut rig.state, KeyRemapState::Restarting).await;
    rig.notices.try_recv().unwrap_err();
    lock(&shared).active_state = "failed".to_owned();
    until(&mut rig.state, KeyRemapState::Failed).await;
    assert!(
        tokio::time::timeout(LONG, rig.notices.recv())
            .await
            .is_ok_and(|n| matches!(n, Some(NotificationsCommand::Post(_))))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn retry_resets_the_failure_then_restarts_the_unit() {
    let (bus, _systemd, shared) = with_systemd(true, "inactive").await;
    let rig = rig(BusRoute::Address(bus.address().to_owned()));
    rig.control
        .send(KeyRemapCommand::Retry)
        .unwrap_or_else(|e| panic!("{e}"));
    let writes = || {
        lock(&shared)
            .calls
            .iter()
            .filter(|c| !c.starts_with("GetUnit"))
            .cloned()
            .collect::<Vec<_>>()
    };
    base_testkit::eventually("the unit was reset and restarted", || writes().len() >= 2).await;
    assert_eq!(
        writes(),
        [
            "ResetFailedUnit(bardo-key-remap.service)",
            "RestartUnit(bardo-key-remap.service, replace)"
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn without_a_system_bus_nothing_is_asked_and_the_state_stays_unknown() {
    // A bus with a stand-in exists, but the service is not told of it.
    let (_bus, _systemd, shared) = with_systemd(true, "failed").await;
    let mut rig = rig(BusRoute::Absent);
    rig.control
        .send(KeyRemapCommand::Retry)
        .unwrap_or_else(|e| panic!("{e}"));
    tokio::time::sleep(SHORT * 2).await;
    assert_eq!(*rig.state.borrow(), KeyRemapState::Unknown);
    rig.notices.try_recv().unwrap_err();
    assert_eq!(lock(&shared).calls, [] as [std::string::String; 0]);
}
