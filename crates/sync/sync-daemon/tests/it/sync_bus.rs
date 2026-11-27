//! `org.bardo.Sync1` on a private bus: who may call, what a name that has no dataset answers,
//! `Status` with its quota, the pause switch, the unicast signals, and the introspection the
//! frozen XML pins. One test runs a real engine over `MemoryReplica` and reads it over the bus.

use crate::common;

use account_capability::{Access, Delta, HashKind, StorageCap, StorageScope};
use base_ids::Bytes;
use base_testkit::PrivateBus;
use base_time::UnixSeconds;
use caller_role_types::CallerRole;
use common::{ACCESS_DENIED, INVALID_ARGS, Known, NO_FITTING, client, error_name, eventually};
use std::collections::BTreeSet;
use std::pin::Pin;
use std::sync::Arc;
use sync_bus::SyncProxy;
use sync_bus::{STATUS_KEY_QUOTA, SYNC_ERROR_PAUSED};
use sync_core::{
    BaseVersion, Conflict, ConflictRule, LocalId, MemoryReplica, Quota, RemoteId, RemoteSide,
    RemoteVersion, StoredConflict,
};
use sync_daemon::service::serve;
use sync_driver::Driver;
use sync_driver::clock::Wall;
use sync_driver::hub::{Access as Visible, DatasetName, Event, Hub, StatusSnapshot};
use sync_engine::dataset::MemoryDataset;
use sync_engine::engine::Engine;
use sync_engine::scheduler::{Network, Settings};
use sync_journal_sqlite::Journal;
use tokio::sync::{Notify, watch};
use zbus::export::futures_core::Stream;
use zbus::zvariant::OwnedValue;

struct Rig {
    bus: PrivateBus,
    known: Known,
    hub: Hub,
    _server: zbus::Connection,
}

async fn rig() -> Rig {
    let bus = PrivateBus::start();
    let known = Known::default();
    let hub = Hub::default();
    let server = bus.connect().await;
    serve(&server, hub.clone(), known.clone())
        .await
        .expect("serve");
    Rig {
        bus,
        known,
        hub,
        _server: server,
    }
}

fn name(text: &str) -> DatasetName {
    DatasetName::parse(text).expect("dataset name")
}

fn owned_by(app: &str) -> Visible {
    Visible {
        owners: BTreeSet::from([account_core::AppName::parse(app).expect("name")]),
    }
}

async fn proxy(connection: &zbus::Connection) -> SyncProxy<'_> {
    SyncProxy::new(connection).await.expect("proxy")
}

#[tokio::test(flavor = "multi_thread")]
async fn a_sender_nothing_names_is_access_denied_and_a_named_one_may_ask() {
    let rig = rig().await;
    let stranger = rig.bus.connect().await;
    let err = proxy(&stranger)
        .await
        .datasets()
        .await
        .expect_err("unknown caller");
    assert_eq!(error_name(&err), ACCESS_DENIED);
    let known = client(&rig.bus, &rig.known, "org.example.App", CallerRole::App).await;
    assert_eq!(
        proxy(&known).await.datasets().await.expect("datasets"),
        Vec::<String>::new()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn with_no_dataset_every_name_answers_the_refusal_and_a_bad_name_is_invalid() {
    let rig = rig().await;
    let me = client(&rig.bus, &rig.known, "org.example.App", CallerRole::App).await;
    let sync = proxy(&me).await;
    assert_eq!(
        sync.datasets().await.expect("datasets"),
        [] as [std::string::String; 0]
    );
    for dataset in ["acct/pim", "a1/photos_originals"] {
        for err in [
            sync.status(dataset).await.map(drop).expect_err("status"),
            sync.pause(dataset).await.expect_err("pause"),
            sync.resume(dataset).await.expect_err("resume"),
            sync.sync_now(dataset).await.expect_err("sync now"),
        ] {
            assert_eq!(error_name(&err), NO_FITTING, "{dataset}");
        }
    }
    for bad in ["", "pim", "a/b/c", "../x/y", "A/B"] {
        let err = sync.status(bad).await.map(drop).expect_err("bad name");
        assert_eq!(error_name(&err), INVALID_ARGS, "{bad:?}");
    }
}

#[tokio::test(flavor = "multi_thread")]
#[expect(
    clippy::too_many_lines,
    reason = "a scenario test tells one story in order; splitting it would hide the sequence"
)]
async fn a_caller_sees_what_it_owns_settings_sees_all_and_status_carries_quota_and_the_pause() {
    let rig = rig().await;
    let photos = rig
        .hub
        .register(name("a1/photos_originals"), owned_by("org.bardo.Photos"));
    let _pim = rig
        .hub
        .register(name("a1/pim"), owned_by("org.bardo.Shell"));
    photos.publish(StatusSnapshot {
        anchor_age: Some(12),
        pending: 3,
        conflicts: 1,
        quota: Some(Quota {
            used: Bytes(10),
            total: Some(Bytes(100)),
        }),
        ..StatusSnapshot::default()
    });
    let app = client(&rig.bus, &rig.known, "org.bardo.Photos", CallerRole::App).await;
    let settings = client(
        &rig.bus,
        &rig.known,
        "org.bardo.Settings",
        CallerRole::Settings,
    )
    .await;
    assert_eq!(
        proxy(&app).await.datasets().await.expect("datasets"),
        ["a1/photos_originals"]
    );
    assert_eq!(
        proxy(&settings).await.datasets().await.expect("datasets"),
        ["a1/photos_originals", "a1/pim"]
    );

    let status = proxy(&app)
        .await
        .status("a1/photos_originals")
        .await
        .expect("status");
    assert_eq!(u64::try_from(&status["pending"]).ok(), Some(3));
    assert_eq!(u64::try_from(&status["conflicts"]).ok(), Some(1));
    assert_eq!(i64::try_from(&status["anchor_age"]).ok(), Some(12));
    assert_eq!(bool::try_from(&status["paused"]).ok(), Some(false));
    let quota: std::collections::HashMap<String, OwnedValue> = status[STATUS_KEY_QUOTA]
        .try_clone()
        .expect("clone")
        .try_into()
        .expect("a{sv}");
    assert_eq!(u64::try_from(&quota["used"]).ok(), Some(10));
    assert_eq!(u64::try_from(&quota["total"]).ok(), Some(100));

    let err = proxy(&app)
        .await
        .status("a1/pim")
        .await
        .map(drop)
        .expect_err("not theirs");
    assert_eq!(
        error_name(&err),
        NO_FITTING,
        "an invisible dataset looks like none"
    );
    let err = proxy(&app)
        .await
        .pause("a1/pim")
        .await
        .expect_err("not theirs");
    assert_eq!(error_name(&err), NO_FITTING);
    let err = proxy(&app)
        .await
        .sync_now("a1/pim")
        .await
        .expect_err("not theirs");
    assert_eq!(error_name(&err), NO_FITTING);
    // Its owner and Settings may ask for it to sync now; a bad name is the caller's to fix.
    for who in [&app, &settings] {
        proxy(who)
            .await
            .sync_now("a1/photos_originals")
            .await
            .expect("sync now");
    }
    let err = proxy(&app)
        .await
        .sync_now("photos")
        .await
        .expect_err("a bad name");
    assert_eq!(error_name(&err), INVALID_ARGS);

    proxy(&app)
        .await
        .pause("a1/photos_originals")
        .await
        .expect("pause");
    for who in [&app, &settings] {
        let err = proxy(who)
            .await
            .sync_now("a1/photos_originals")
            .await
            .expect_err("paused");
        assert_eq!(error_name(&err), SYNC_ERROR_PAUSED);
    }
    assert_eq!(photos.pausing(), sync_engine::scheduler::Pausing::Paused);
    let status = proxy(&settings)
        .await
        .status("a1/photos_originals")
        .await
        .expect("status");
    assert_eq!(bool::try_from(&status["paused"]).ok(), Some(true));
    proxy(&settings)
        .await
        .resume("a1/photos_originals")
        .await
        .expect("resume");
    assert_eq!(photos.pausing(), sync_engine::scheduler::Pausing::Running);
}

async fn next<S: Stream + Unpin>(stream: &mut S) -> S::Item {
    tokio::time::timeout(
        base_testkit::GENEROUS,
        std::future::poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx)),
    )
    .await
    .expect("a signal in time")
    .expect("the stream is open")
}

#[tokio::test(flavor = "multi_thread")]
async fn progress_and_conflict_go_only_to_callers_who_may_see_the_dataset() {
    let rig = rig().await;
    let photos = rig
        .hub
        .register(name("a1/photos_originals"), owned_by("org.bardo.Photos"));
    let pim = rig
        .hub
        .register(name("a1/pim"), owned_by("org.bardo.Shell"));
    let app = client(&rig.bus, &rig.known, "org.bardo.Photos", CallerRole::App).await;
    let shell = client(&rig.bus, &rig.known, "org.bardo.Shell", CallerRole::App).await;
    let (app_proxy, shell_proxy) = (proxy(&app).await, proxy(&shell).await);
    let mut app_progress = app_proxy.receive_progress().await.expect("stream");
    let mut shell_progress = shell_proxy.receive_progress().await.expect("stream");
    let mut app_conflict = app_proxy.receive_conflict().await.expect("stream");
    // Calling joins the roster: the daemon tells only connections it has heard from.
    app_proxy.datasets().await.expect("datasets");
    shell_proxy.datasets().await.expect("datasets");

    photos.tell(Event::Progress {
        dataset: name("a1/photos_originals"),
        fetched: 2,
        uploaded: 1,
    });
    pim.tell(Event::Progress {
        dataset: name("a1/pim"),
        fetched: 7,
        uploaded: 0,
    });
    photos.tell(Event::Conflict {
        dataset: name("a1/photos_originals"),
        conflict: Box::new(StoredConflict {
            number: Some(1),
            conflict: Conflict {
                item: RemoteId("r1".into()),
                base: BaseVersion::Absent,
                remote: RemoteSide::Exists(RemoteId("r1".into()), RemoteVersion("v2".into())),
            },
            local: LocalId("IMG_1.HEIC".into()),
            local_hash: None,
            at: UnixSeconds(5),
        }),
    });

    let signal = next(&mut app_progress).await;
    assert_eq!(
        signal.args().expect("args").dataset(),
        &"a1/photos_originals"
    );
    let signal = next(&mut shell_progress).await;
    assert_eq!(
        signal.args().expect("args").dataset(),
        &"a1/pim",
        "Shell never hears of the Photos dataset"
    );
    let conflict = next(&mut app_conflict).await;
    let args = conflict.args().expect("args");
    assert_eq!(args.dataset(), &"a1/photos_originals");
    assert!(args.conflict().contains_key("remote"));
}

fn storage() -> StorageCap {
    StorageCap {
        access: Access::ReadWrite,
        delta: Delta::Poll,
        quota: true,
        scope: StorageScope::AppFolder,
        hashes: HashKind::Sha256,
        ranges: true,
        chunked_upload: false,
    }
}

fn quick() -> Settings {
    Settings::quick()
}

async fn used(sync: &SyncProxy<'_>, dataset: &str) -> Option<u64> {
    let status = sync.status(dataset).await.ok()?;
    let quota: std::collections::HashMap<String, OwnedValue> = status
        .get(STATUS_KEY_QUOTA)?
        .try_clone()
        .ok()?
        .try_into()
        .ok()?;
    u64::try_from(&quota["used"]).ok()
}

/// Polls `Status` until the replica's used bytes are `want` (the journal syncs to disk on every
/// write, and a loaded machine is slow).
async fn used_reaches(sync: &SyncProxy<'_>, dataset: &str, want: u64) {
    base_testkit::eventually_some_async(
        "the upload to show as the replica's used bytes",
        || async { (used(sync, dataset).await == Some(want)).then_some(()) },
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_running_engine_syncs_to_its_replica_and_the_bus_shows_it_until_it_is_paused() {
    let rig = rig().await;
    let dir = rig.bus.scratch().join("journal");
    let dataset = Arc::new(MemoryDataset::new(
        "photos_originals",
        ConflictRule::Impossible,
    ));
    let engine = Engine::new(
        MemoryReplica::new(storage(), 10, UnixSeconds(1)),
        Arc::clone(&dataset),
        Journal::open(&dir.join("photos_originals.sqlite")).expect("journal"),
        Wall::scaled(quick().time_scale),
    );
    let handle = rig
        .hub
        .register(name("a1/photos_originals"), owned_by("org.bardo.Photos"));
    let (_net, network) = watch::channel(Network::Unmetered);
    let driver = Driver::new(engine, handle, quick(), network, Arc::new(Notify::new()), 7);
    let running = tokio::spawn(driver.run());

    let app = client(&rig.bus, &rig.known, "org.bardo.Photos", CallerRole::App).await;
    let sync = proxy(&app).await;
    dataset.put("IMG_1.HEIC", b"abc");
    sync.sync_now("a1/photos_originals")
        .await
        .expect("sync now");
    used_reaches(&sync, "a1/photos_originals", 3).await;
    let status = sync.status("a1/photos_originals").await.expect("status");
    assert_eq!(u64::try_from(&status["pending"]).ok(), Some(0));
    assert!(status.contains_key("anchor_age"));

    sync.pause("a1/photos_originals").await.expect("pause");
    // A cycle that began before the pause finishes (the pause is read between cycles): wait for
    // it to end, not for a time. One that was only waiting to begin sees the pause and skips.
    drop(
        rig.hub
            .hold_cycles(&name("a1/photos_originals"))
            .await
            .expect("the dataset is running"),
    );
    dataset.put("IMG_2.HEIC", b"defg");
    // Sync now does not get past the pause; it says so.
    let err = sync
        .sync_now("a1/photos_originals")
        .await
        .expect_err("paused");
    assert_eq!(error_name(&err), SYNC_ERROR_PAUSED);
    tokio::time::sleep(common::poll_time(1500)).await;
    assert_eq!(
        used(&sync, "a1/photos_originals").await,
        Some(3),
        "paused: nothing went up"
    );
    sync.resume("a1/photos_originals").await.expect("resume");
    // Sync now starts the cycle that sends the second photo without waiting for a look.
    sync.sync_now("a1/photos_originals")
        .await
        .expect("sync now");
    used_reaches(&sync, "a1/photos_originals", 7).await;

    // Removing the account's datasets from the hub ends the driver.
    assert!(
        !rig.hub
            .forget_account(&sync_xdg::AccountDir::parse("a1").expect("account"))
            .is_empty(),
        "the account had datasets"
    );
    eventually("the driver stops", || running.is_finished()).await;
}
