//! The client against the real session server: `Open` over a private bus returns a socket that
//! `inference_daemon::serve` serves with the real session machine over scripted seams. What the machine
//! adds to the scripts (`Routed`, `Waiting`, refusals, the computer-use begin) reaches the client
//! over the bus exactly as inferd will send it.
#![cfg(feature = "dbus")]

use crate::common;

use account_capability::ComputerUseEnv;
use account_capability::ModelId;
use account_capability::{Locality, Tier};
use account_core::AccountId;
use account_core::DataClass;
use account_core::consent::Usage;
use account_match::Need;
use account_match::{ComputerUseNeed, DimsNeed, EmbedNeed};
use base_ids::{Dims, Tokens};
use base_testkit::PrivateBus;
use common::inferd::{Behaviour, FakeInferd, Served};
use common::served::{Gate, Route, Scripted};
use inference_client::{DbusTransport, InferSession, Inference};
use inference_core::testing::{Script, ScriptStep};
use inference_core::{
    AttachIndex, ClientFrame, ComputerUseBegin, ComputerUseStepReply, EmbedReply, EmbedRequest,
    EmbedRole, EmbedVector, ImageSource, InferEvent, InferRefusedError, InferReply, InferRequest,
    Readiness, RequestKind, ServedBy, TurnUsage,
};
use inference_router::session::RouteDecision;
use std::sync::Arc;
use tokio::sync::Notify;

fn served_by() -> ServedBy {
    ServedBy {
        account: AccountId::parse("local").expect("id"),
        model: ModelId::parse("qwen").expect("id"),
        locality: Locality::OnDevice,
    }
}

fn decision(readiness: Readiness) -> Route {
    Route(Ok(RouteDecision {
        served: served_by(),
        readiness,
    }))
}

fn usage() -> TurnUsage {
    TurnUsage {
        input: Tokens(1),
        output: Tokens(0),
        cached: Tokens(0),
        charge: None,
        ..TurnUsage::default()
    }
}

struct Rig {
    _bus: PrivateBus,
    _daemon: zbus::Connection,
    accounts: Inference<DbusTransport>,
    runner: Scripted,
}

async fn rig(route: Route, gate: Gate, scripts: Vec<Script>) -> Rig {
    let bus = PrivateBus::start();
    let runner = Scripted::new(scripts);
    let (fake, _seen) = FakeInferd::new(Behaviour::Served(Served {
        route,
        gate,
        runner: runner.clone(),
    }));
    let daemon = bus.connect().await;
    fake.serve(&daemon).await;
    Rig {
        accounts: Inference::over(DbusTransport::over(bus.connect().await)),
        _bus: bus,
        _daemon: daemon,
        runner,
    }
}

fn embeddings() -> Need {
    Need::Embeddings(EmbedNeed::new(
        DimsNeed::Exactly(Dims(2)),
        [account_capability::Modality::Text].into(),
    ))
}

fn embed_request() -> InferRequest {
    InferRequest::Embed(EmbedRequest::new(
        vec!["a note".into()],
        EmbedRole::Document,
        DimsNeed::Exactly(Dims(2)),
        DataClass::Notes,
        Usage::Background,
    ))
}

fn embed_reply() -> InferReply {
    InferReply::Embed(EmbedReply::new(
        vec![EmbedVector(vec![0.5, 0.25])],
        usage(),
        served_by(),
    ))
}

fn embed_script() -> Script {
    Script {
        kind: RequestKind::Embed,
        steps: vec![ScriptStep::Emit(InferEvent::Finished(embed_reply()))],
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn memoryds_embedding_turn_runs_through_the_real_machine_and_says_who_answered() {
    let rig = rig(
        decision(Readiness::Ready),
        Gate::default(),
        vec![embed_script()],
    )
    .await;
    let mut session = rig
        .accounts
        .session(&embeddings(), DataClass::Notes, Tier::Fast)
        .await
        .expect("open");
    session
        .send(ClientFrame::Request(embed_request()))
        .await
        .expect("send");
    assert_eq!(session.next().await, Ok(InferEvent::Routed(served_by())));
    assert_eq!(
        session.next().await,
        Ok(InferEvent::Finished(embed_reply()))
    );
}

async fn rig_again(scripts: Vec<Script>) -> Rig {
    rig(decision(Readiness::Ready), Gate::default(), scripts).await
}

#[tokio::test(flavor = "multi_thread")]
async fn a_class_the_session_was_not_opened_for_is_refused_by_the_machine() {
    let rig = rig_again(vec![embed_script()]).await;
    let mut session = rig
        .accounts
        .session(&embeddings(), DataClass::Public, Tier::Fast)
        .await
        .expect("open");
    session
        .send(ClientFrame::Request(embed_request()))
        .await
        .expect("send");
    assert_eq!(
        session.next().await,
        Ok(InferEvent::Finished(InferReply::Refused(
            InferRefusedError::Unsupported
        ))),
        "the request carries Notes on a Public session"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_loading_engine_is_a_waiting_event_until_it_is_ready() {
    let gate = Arc::new(Notify::new());
    let rig = rig(
        decision(Readiness::Loadable),
        Gate(Some(Arc::clone(&gate))),
        vec![embed_script()],
    )
    .await;
    let mut session = rig
        .accounts
        .session(&embeddings(), DataClass::Notes, Tier::Fast)
        .await
        .expect("open");
    assert_eq!(
        session.next().await,
        Ok(InferEvent::Waiting(Readiness::Loadable))
    );
    session
        .send(ClientFrame::Request(embed_request()))
        .await
        .expect("send");
    gate.notify_one();
    assert_eq!(session.next().await, Ok(InferEvent::Routed(served_by())));
    assert_eq!(
        session.next().await,
        Ok(InferEvent::Finished(embed_reply()))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_computer_use_run_needs_its_begin_and_its_frame_rides_as_a_memfd() {
    let ack = InferReply::ComputerUseStep(ComputerUseStepReply::new(vec![]));
    let step_reply = InferReply::ComputerUseStep(
        ComputerUseStepReply::new(vec![computer_use_action::ComputerUseAction::Observe])
            .with_thought("done".into()),
    );
    let scripts = vec![
        Script {
            kind: RequestKind::ComputerUseBegin,
            steps: vec![ScriptStep::Emit(InferEvent::Finished(ack.clone()))],
        },
        Script {
            kind: RequestKind::ComputerUseStep,
            steps: vec![ScriptStep::Emit(InferEvent::Finished(step_reply.clone()))],
        },
    ];
    let rig = rig_again(scripts).await;
    let need = Need::ComputerUse(ComputerUseNeed::new([ComputerUseEnv::Desktop].into()));
    let mut session = rig
        .accounts
        .session(&need, DataClass::Screen, Tier::Best)
        .await
        .expect("open");
    let frame =
        || rustix::fs::memfd_create("frame", rustix::fs::MemfdFlags::CLOEXEC).expect("memfd");
    let step = ClientFrame::Request(InferRequest::ComputerUseStep(step_request()));

    session
        .send_attached(step.clone(), vec![frame()])
        .await
        .expect("send");
    assert_eq!(
        session.next().await,
        Ok(InferEvent::Finished(InferReply::Refused(
            InferRefusedError::Unsupported
        ))),
        "a step before a begin"
    );

    session
        .send(ClientFrame::Request(InferRequest::ComputerUseBegin(
            ComputerUseBegin::new("rename".into(), ComputerUseEnv::Desktop),
        )))
        .await
        .expect("send");
    assert_eq!(session.next().await, Ok(InferEvent::Routed(served_by())));
    assert_eq!(session.next().await, Ok(InferEvent::Finished(ack)));

    session
        .send_attached(step, vec![frame()])
        .await
        .expect("send");
    assert_eq!(session.next().await, Ok(InferEvent::Finished(step_reply)));
    let seen = rig.runner.seen.lock().expect("lock");
    let kinds: Vec<(RequestKind, usize)> = seen.iter().map(|(r, n)| (r.kind(), *n)).collect();
    assert_eq!(
        kinds,
        vec![
            (RequestKind::ComputerUseBegin, 0),
            (RequestKind::ComputerUseStep, 1)
        ],
        "the engine saw the begin, then the step with its one descriptor"
    );
}

fn step_request() -> inference_core::ComputerUseStepRequest {
    inference_core::ComputerUseStepRequest::new(
        inference_core::StepIndex(0),
        inference_core::WindowGeometry {
            logical: computer_use_action::Size::new(
                computer_use_action::Coord(1),
                computer_use_action::Coord(1),
            ),
            scale: computer_use_action::Scale120(120),
        },
        inference_core::FrameImage {
            source: ImageSource::Attached(AttachIndex(0)),
            layout: inference_core::FrameLayout::Encoded(inference_core::MediaType::Png),
        },
        None,
    )
}
