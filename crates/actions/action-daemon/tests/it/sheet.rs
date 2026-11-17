//! The sheet over a real bus: the shell's `Confirm1` is asked only when its owner is the process the
//! configuration trusts with the `confirm` role; whatever goes wrong with the person's side (no
//! The shell, a shell nobody trusts, one that never answers, one that vanishes) is a dismissal or an
//! expiry, never an allow.

use crate::support::apps::{Answer, FakeShell};
use crate::support::bus::PrivateBus;
use crate::support::world::{app, introduce};
use action_core::*;
use action_daemon::{IntentdConfig, SheetConfirmer};
use action_dbus::callers::Identity;
use action_vocab::{LabelText, Seconds};
use base_ids::Count;
use caller_role_types::CallerRole;
use permission_core::{ACP_AGENT_APP, AskReason};
use provenance::{Actor, ConfirmReceipt, Effect, InputProof, SpaceId, UnixSeconds};
use std::sync::Arc;
use std::time::Duration;

fn request(id: &str, expires: u32) -> ConfirmRequest {
    ConfirmRequest {
        id: ConfirmId::parse(id).expect("id"),
        space: SpaceId::parse("work").expect("space"),
        actor: Actor::Cli,
        app: app("org.bardo.Mail"),
        action: LabelText::parse("Archive a thread").expect("label"),
        effect: Effect::UndoableWrite,
        count: Count(1),
        detail: ConfirmDetail::Plain,
        lines: vec![],
        why: vec![AskReason::FromTerminal],
        taint: TaintNote::Clean,
        offer: ConfirmOffer::OnceOrFromTerminal,
        always: permission_core::AlwaysOffer::default(),
        limits: SheetLimits::default(),
        gesture: Gesture::Press,
        anchor: Anchor::Centre,
        expires: Seconds(expires),
        editor: None,
        helper: None,
        brief: None,
    }
}

fn receipt() -> ConfirmReceipt {
    ConfirmReceipt {
        id: ConfirmId::parse("c-1").expect("id"),
        input: InputProof::HardwareSeat,
        at: UnixSeconds(1),
        covers: provenance::Confidentiality::Secret,
    }
}

fn yes() -> ConfirmAnswer {
    ConfirmAnswer::Allowed {
        written: None,
        scope: GrantScope::Once,
        receipt: receipt(),
    }
}

struct Desk {
    _dir: tempfile::TempDir,
    bus: PrivateBus,
    sheet: SheetConfirmer,
    who: Arc<Identity>,
    config: IntentdConfig,
}

/// The table intentd reads, over a fake proc root in which this test process is in a unit nothing
/// names: a connection is nobody unless the test introduces it.
fn identity(dir: &tempfile::TempDir, daemon: &action_dbus::BusConnection) -> Arc<Identity> {
    Arc::new(Identity::on_fixture(
        daemon.clone(),
        action_daemon::callers::shipped(),
        dir.path().join("proc"),
    ))
}

async fn desk() -> Desk {
    let dir = tempfile::tempdir().expect("scratch");
    let bus = PrivateBus::start(dir.path());
    let daemon = bus.connect().await;
    let config = IntentdConfig::shipped().expect("the shipped configuration");
    let who = identity(&dir, &daemon);
    let sheet = SheetConfirmer::trusting(daemon, Arc::new(config.clone()), Arc::clone(&who))
        .waiting(Duration::from_millis(100));
    Desk {
        _dir: dir,
        bus,
        sheet,
        who,
        config,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_trusted_shell_is_asked_and_its_answer_is_the_answer() {
    let desk = desk().await;
    let connection = desk.bus.connect().await;
    // The shipped configuration gives the shell the name org.bardo.Shell and the confirm role.
    let shell = FakeShell::start(&connection, &["org.bardo.Confirm1", "org.bardo.Shell"]).await;
    introduce(
        &desk.who,
        &connection,
        &desk.config,
        &["org.bardo.Confirm1", "org.bardo.Shell"],
    );
    shell.answer(vec![
        Answer::With(yes()),
        Answer::With(ConfirmAnswer::AllowedFromTerminal { receipt: receipt() }),
        Answer::With(ConfirmAnswer::Ended(ConfirmEnd::Refused)),
    ]);
    assert_eq!(desk.sheet.confirm(request("c-1", 120)).await, yes());
    assert_eq!(
        desk.sheet.confirm(request("c-2", 120)).await,
        ConfirmAnswer::AllowedFromTerminal { receipt: receipt() }
    );
    assert_eq!(
        desk.sheet.confirm(request("c-3", 120)).await,
        ConfirmAnswer::Ended(ConfirmEnd::Refused)
    );
    let shown = shell.shown();
    assert_eq!(shown.len(), 3);
    assert_eq!(
        shown[0],
        request("c-1", 120),
        "the whole request arrives, typed"
    );
    // Nothing queued: the person walks away.
    assert_eq!(
        desk.sheet.confirm(request("c-4", 120)).await,
        ConfirmAnswer::Ended(ConfirmEnd::Dismissed)
    );
    desk.sheet
        .cancel(&ConfirmId::parse("c-4").expect("id"))
        .await;
    assert_eq!(shell.cancelled(), ["c-4"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_name_nobody_trusted_is_never_asked_and_never_allows() {
    let desk = desk().await;
    let connection = desk.bus.connect().await;
    // Owns org.bardo.Confirm1 but plays no role in the configuration.
    let impostor = FakeShell::start(&connection, &["org.bardo.Confirm1"]).await;
    introduce(
        &desk.who,
        &connection,
        &desk.config,
        &["org.bardo.Confirm1"],
    );
    impostor.answer(vec![Answer::With(yes())]);
    assert_eq!(
        desk.sheet.confirm(request("c-1", 120)).await,
        ConfirmAnswer::Ended(ConfirmEnd::Dismissed),
        "an answer from a process that is not the shell is no answer"
    );
    assert!(impostor.shown().is_empty(), "the request never reached it");
    desk.sheet
        .cancel(&ConfirmId::parse("c-1").expect("id"))
        .await;
    assert!(impostor.cancelled().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_shell_is_a_dismissal() {
    let desk = desk().await;
    assert_eq!(
        desk.sheet.confirm(request("c-1", 120)).await,
        ConfirmAnswer::Ended(ConfirmEnd::Dismissed)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_sheet_nobody_answers_expires_and_a_shell_that_vanishes_dismisses() {
    let desk = desk().await;
    let connection = desk.bus.connect().await;
    let shell = FakeShell::start(&connection, &["org.bardo.Confirm1", "org.bardo.Shell"]).await;
    introduce(
        &desk.who,
        &connection,
        &desk.config,
        &["org.bardo.Confirm1", "org.bardo.Shell"],
    );
    shell.answer(vec![Answer::Never, Answer::Never]);
    // Its own expiry (0 s) plus the grace: expired, not allowed.
    assert_eq!(
        desk.sheet.confirm(request("c-1", 0)).await,
        ConfirmAnswer::Ended(ConfirmEnd::Expired)
    );
    // The sheet is up and the shell goes away: dismissed, at once.
    let waiting = tokio::spawn({
        let sheet = desk.sheet.clone();
        async move { sheet.confirm(request("c-2", 120)).await }
    });
    base_testkit::eventually("the second sheet to be shown", || {
        shell
            .shown()
            .iter()
            .any(|shown| shown.id.to_string() == "c-2")
    })
    .await;
    connection.close().await.expect("close shell");
    let ended = tokio::time::timeout(Duration::from_secs(5), waiting)
        .await
        .expect("it does not hang")
        .expect("joined");
    assert_eq!(ended, ConfirmAnswer::Ended(ConfirmEnd::Dismissed));
}

/// A sheet for a session whose host asked to show it itself (`bardo-agent --tty`).
fn hosted(id: &str) -> ConfirmRequest {
    ConfirmRequest {
        editor: Some(EditorRoute {
            client: provenance::ClientName::parse(ACP_AGENT_APP).expect("client"),
            session: provenance::SessionId::parse("s-1").expect("session"),
        }),
        ..request(id, 120)
    }
}

async fn gated_desk(agents: bool) -> Desk {
    let dir = tempfile::tempdir().expect("scratch");
    let bus = PrivateBus::start(dir.path());
    let daemon = bus.connect().await;
    let config = IntentdConfig::shipped().expect("the shipped configuration");
    let gate = action_daemon::AcpGate::shut();
    gate.set_agents(agents);
    let who = identity(&dir, &daemon);
    let sheet = SheetConfirmer::trusting(daemon, Arc::new(config.clone()), Arc::clone(&who))
        .gated(
            gate,
            &action_daemon::ProcRoot::Honoured(dir.path().join("proc")),
        )
        .waiting(Duration::from_millis(100));
    Desk {
        _dir: dir,
        bus,
        sheet,
        who,
        config,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_sheet_the_agent_host_asked_for_goes_to_its_confirm1_and_not_to_shell() {
    let desk = gated_desk(true).await;
    let (apart, other) = (desk.bus.connect().await, desk.bus.connect().await);
    let shell = FakeShell::start(&apart, &["org.bardo.Confirm1", "org.bardo.Shell"]).await;
    introduce(
        &desk.who,
        &apart,
        &desk.config,
        &["org.bardo.Confirm1", "org.bardo.Shell"],
    );
    let host = FakeShell::start(&other, &[ACP_AGENT_APP]).await;
    introduce(&desk.who, &other, &desk.config, &[ACP_AGENT_APP]);
    host.answer(vec![Answer::With(yes())]);
    assert_eq!(desk.sheet.confirm(hosted("c-1")).await, yes());
    assert_eq!(host.shown().len(), 1);
    assert!(shell.shown().is_empty(), "the desktop was not asked");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_sheet_for_an_agent_host_that_is_off_or_gone_expires_and_is_not_moved_to_shell() {
    // The setting is off: the name is nobody, whoever owns it.
    let off = gated_desk(false).await;
    let (apart, other) = (off.bus.connect().await, off.bus.connect().await);
    let shell = FakeShell::start(&apart, &["org.bardo.Confirm1", "org.bardo.Shell"]).await;
    introduce(
        &off.who,
        &apart,
        &off.config,
        &["org.bardo.Confirm1", "org.bardo.Shell"],
    );
    let host = FakeShell::start(&other, &[ACP_AGENT_APP]).await;
    introduce(&off.who, &other, &off.config, &[ACP_AGENT_APP]);
    host.answer(vec![Answer::With(yes())]);
    assert_eq!(
        off.sheet.confirm(hosted("c-1")).await,
        ConfirmAnswer::Ended(ConfirmEnd::Expired)
    );
    assert!(host.shown().is_empty());
    // The setting is on and nobody owns the name.
    let on = gated_desk(true).await;
    let connection = on.bus.connect().await;
    let shell_on = FakeShell::start(&connection, &["org.bardo.Confirm1", "org.bardo.Shell"]).await;
    introduce(
        &on.who,
        &connection,
        &on.config,
        &["org.bardo.Confirm1", "org.bardo.Shell"],
    );
    assert_eq!(
        on.sheet.confirm(hosted("c-2")).await,
        ConfirmAnswer::Ended(ConfirmEnd::Expired)
    );
    assert!(shell.shown().is_empty() && shell_on.shown().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_process_that_is_not_the_shells_unit_is_never_asked() {
    // Each owns org.bardo.Confirm1 and the shell's name and answers yes. A scope that claims the
    // shell is a plain app (the accounts service gives a scope row no fixed role), an agent is not the sheet
    // host, and a process nothing identifies is nobody: none is asked, none allows.
    for (what, standing) in [
        ("a scope claiming the shell", Some(CallerRole::App)),
        ("an agent claiming the shell", Some(CallerRole::Agent)),
        ("an unknown process", None),
    ] {
        let desk = desk().await;
        let connection = desk.bus.connect().await;
        let claimant =
            FakeShell::start(&connection, &["org.bardo.Confirm1", "org.bardo.Shell"]).await;
        if let Some(role) = standing {
            desk.who.introduce_app(&connection, "org.bardo.Shell", role);
        }
        claimant.answer(vec![Answer::With(yes())]);
        assert_eq!(
            desk.sheet.confirm(request("c-1", 120)).await,
            ConfirmAnswer::Ended(ConfirmEnd::Dismissed),
            "{what}"
        );
        assert!(
            claimant.shown().is_empty(),
            "{what}: the request never reached it"
        );
    }
}
