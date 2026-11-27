//! A real actiond over the fake router on a private bus, and the records and asks the link tests send it.

use account_core::AppName;
use action_client::{DbusTransport, Intents};
use action_core::{ConfirmAnswer, GrantScope};
use action_daemon::{AcpGate, ActiondConfig, ProcRoot, serve_on_gated};
use action_dbus::callers::{CallerTable, Identity};
use action_fake::{ScriptedConfirmer, fake_router};
use action_gate_wire::{ComputerUseAsk, EffectBasis, GateAnswer};
use action_router::Router;
use action_vocab::CallerRole;
use action_wire::SessionOpen;
use base_testkit::PrivateBus;
use caller_role_types::CallerRole as Role;
use computer_use_action::ComputerUseAction;
use computer_use_action::{RunMode, WindowTrust};
use computer_use_bus::{
    ActionResult, AuthorisedBy, ComputerUseOutcome, ComputerUseRecord, FrameNote, TargetedAction,
    ThoughtDigest, WindowRef,
};
use computer_use_capture::FrameDigest;
use computer_use_daemon::IntentsGate;
use computer_use_desktop::Digest32;
use computer_use_geometry::WindowSpace;
use computer_use_run::{GateCheck, GateEvent, GateLink};
use provenance::{
    AgentRef, ConfirmReceipt, Effect, InputProof, Label, RunId, SessionId, Source, SpaceId,
    UnixSeconds,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use zbus::Connection;

pub(crate) const COMPANION: &str = "org.bardo.Companion1";
pub(crate) const COMPUTER_USE_DAEMON: &str = "org.bardo.ComputerUse1";

pub(crate) fn mail() -> AppName {
    AppName::parse("org.bardo.Mail").unwrap()
}

pub(crate) fn work() -> SpaceId {
    SpaceId::parse("work").unwrap()
}

pub(crate) fn run() -> RunId {
    RunId::parse("r-1").unwrap()
}

pub(crate) fn yes() -> ConfirmAnswer {
    ConfirmAnswer::Allowed {
        scope: GrantScope::Once,
        receipt: ConfirmReceipt {
            id: provenance::ConfirmId::parse("c-1").unwrap(),
            input: InputProof::HardwareSeat,
            at: UnixSeconds(1),
            covers: provenance::Confidentiality::Secret,
        },
        written: None,
    }
}

/// A real actiond over the fake router, on a private bus.
pub(crate) struct Desk {
    _dir: tempfile::TempDir,
    bus: PrivateBus,
    who: Arc<Identity>,
    _daemon: Connection,
}

pub(crate) async fn desk(answers: Vec<ConfirmAnswer>) -> Desk {
    let dir = tempfile::tempdir().expect("scratch");
    let bus = PrivateBus::start_in(dir.path());
    let daemon = bus.connect().await;
    let mut router: Router<action_fake::FakeSeams> =
        fake_router(action_core::AgentConfig::default()).expect("a router");
    router.seams.confirmer = ScriptedConfirmer::answering(answers);
    let mut roles = BTreeMap::new();
    roles.insert(
        CallerRole::Companion,
        vec![AppName::parse(COMPANION).unwrap()],
    );
    roles.insert(
        CallerRole::ComputerUse,
        vec![AppName::parse(COMPUTER_USE_DAEMON).unwrap()],
    );
    let config = ActiondConfig {
        roles,
        reviewers: None,
        agent: action_core::AgentConfig::default(),
    };
    // Nothing is read from a real `/proc`: the tree is empty and every caller is introduced.
    let proc_root = dir.path().join("proc");
    let who = Arc::new(Identity::on_fixture(
        daemon.clone(),
        CallerTable::default(),
        proc_root.clone(),
    ));
    serve_on_gated(
        &daemon,
        Arc::new(router),
        Arc::new(config),
        Arc::clone(&who),
        &ProcRoot::Honoured(proc_root),
        AcpGate::shut(),
        None,
    )
    .await
    .expect("actiond serves");
    Desk {
        _dir: dir,
        bus,
        who,
        _daemon: daemon,
    }
}

impl Desk {
    pub(crate) async fn as_companion(&self) -> Intents<DbusTransport> {
        let connection = self.bus.connect().await;
        connection.request_name(COMPANION).await.expect("a name");
        self.who.introduce_app(&connection, COMPANION, Role::Agent);
        // The connection is kept alive by the transport.
        Intents::over(DbusTransport::new(connection))
    }

    pub(crate) async fn computer_use_daemon_connection(&self) -> Connection {
        let connection = self.bus.connect().await;
        connection
            .request_name(COMPUTER_USE_DAEMON)
            .await
            .expect("a name");
        self.who
            .introduce_app(&connection, COMPUTER_USE_DAEMON, Role::ComputerUse);
        connection
    }

    /// What the planner does when it starts a run: opens the run's session, and its own to
    /// hear back on.
    pub(crate) async fn open_sessions(&self, planner: &Intents<DbusTransport>) -> SessionId {
        let hearing = planner
            .session_open(SessionOpen {
                space: work(),
                agent: AgentRef::Companion,
                parent: None,
                cwd: None,
                external: None,
                helper: None,
                started_from: None,
                acts_for: None,
            })
            .await
            .expect("the planner's session");
        let _ = hearing;
        planner
            .session_open(SessionOpen {
                space: work(),
                agent: AgentRef::ComputerUse { run: run() },
                parent: None,
                cwd: None,
                external: None,
                helper: None,
                started_from: None,
                acts_for: None,
            })
            .await
            .expect("the run's session")
            .session
    }
}

pub(crate) fn step(effect: Effect) -> ComputerUseAsk {
    ComputerUseAsk {
        run: run(),
        step: 1,
        app: mail(),
        trust: WindowTrust::Bardo,
        mode: RunMode::InPlace,
        space: work(),
        action: ComputerUseAction::<WindowSpace>::Observe,
        node: None,
        effect,
        basis: EffectBasis::DefaultTable,
        screen: Label::untrusted(
            Source::Screen { app: mail() },
            account_core::DataClass::Mail,
            work(),
        ),
    }
}

pub(crate) async fn answered(gate: &IntentsGate<DbusTransport>, ask: ComputerUseAsk) -> GateAnswer {
    let mut check = gate.check(ask).await.expect("a check");
    loop {
        match check.next().await.expect("an event") {
            GateEvent::Answered(answer) => return answer,
            // The router is about to ask the person: the computer-use daemon's input is idle, so say go on.
            GateEvent::Confirming(_) => check.proceed().await.expect("proceed"),
        }
    }
}

pub(crate) fn record_of_run(run: &RunId) -> Vec<ComputerUseRecord> {
    let action = TargetedAction {
        window: WindowRef(1),
        action: ComputerUseAction::<WindowSpace>::Observe,
    };
    vec![
        ComputerUseRecord::RunStarted {
            run: run.clone(),
            session: SessionId::parse("s-2").unwrap(),
            conversation: SessionId::parse("s-1").unwrap(),
            goal: provenance::Labelled::new("sign in".to_owned(), Label::trusted_user()),
            app: mail(),
            space: work(),
            mode: RunMode::InPlace,
            backend: computer_use_desktop::BackendKind::KwinStock,
            budget: computer_use_bus::RunBudget::PROPOSED,
        },
        ComputerUseRecord::Step {
            run: run.clone(),
            n: inference_core::StepIndex(1),
            thought: ThoughtDigest(Digest32([1; 32])),
            action,
            effect: Effect::Read,
            basis: EffectBasis::DefaultTable,
            authorised_by: AuthorisedBy::Policy,
            result: ActionResult::Done,
            frame: FrameNote::NotKept {
                digest: FrameDigest(Digest32([2; 32])),
                size: computer_use_geometry::DeviceSize { w: 10, h: 10 },
            },
            input: None,
        },
        ComputerUseRecord::Finished {
            run: run.clone(),
            outcome: ComputerUseOutcome::Done,
            steps: base_ids::Count(1),
            charges: inference_core::ChargeTotal::default(),
            report: None,
        },
    ]
}
