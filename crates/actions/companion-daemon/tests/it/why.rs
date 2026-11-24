//! The answer says which model answered, how it was reached and why: inferd's `Why`, `Stage` and
//! `Declined` events become the answer's route notes (`Answer.Routing`), and a refusal of a named
//! model says which and why.

use crate::support::infer::{Say, served, words};
use crate::support::world::*;
use account_callers::ProcRoot;
use account_capability::ModelId;
use account_core::AccountId;
use action_core::*;
use action_dbus::{CompanionAnswerProxy, CompanionProxy, Details};
use action_vocab::{Caller, Origin, WindowKey};
use action_wire::{SessionOpen, SessionOpened};
use base_testkit::PrivateBus;
use base_time::Clock as _;
use companion_daemon::serve_on_rooted;
use companion_wire::{
    AnswerBody, AnswerPhase, AnswerWire, AskWire, RefusalWire, RouteNote, WhyWord, footer_line,
};
use futures_util::StreamExt;
use futures_util::lock::Mutex;
use inference_core::{
    Declined, DeclinedBecause, Door, InferEvent, InferRefusedError, ModelLabel, ModelRef,
    ProviderId, StageNote, StageRole, Why,
};
use provenance::AgentRef;
use std::sync::Arc;
use std::time::Duration;

fn model(id: &str) -> ModelRef {
    ModelRef {
        account: AccountId::parse("local").expect("account"),
        model: ModelId::parse(id).expect("model"),
    }
}

/// What inferd sends when a hosted model is warm and the reasons are shown.
fn hosted() -> Vec<InferEvent> {
    vec![
        InferEvent::Why(Why::Evicted {
            model: model("qwen-3"),
        }),
        InferEvent::Why(Why::Warm),
        InferEvent::Why(Why::Reached {
            provider: ProviderId("openrouter".into()),
            door: Door::Gateway,
        }),
        InferEvent::Routed(served()),
        InferEvent::Stage(StageNote {
            role: StageRole::Answer,
            served: served(),
            why: Why::Warm,
            name: Some(ModelLabel("Scripted One".into())),
        }),
    ]
}

const LINE: &str = "Answered by Scripted One via OpenRouter (unloaded Qwen 3, already loaded)";

#[tokio::test]
async fn the_answer_keeps_the_turns_route_and_the_line_reads_it() {
    let mut w = world(vec![Say::Routed(hosted(), Box::new(words("Done.")))]);
    let opened = w.open("work").await;
    Box::pin(w.say(&opened.session, "hello")).await;
    let notes = w.companion.shared.route_of(&opened.task).expect("notes");
    assert_eq!(footer_line(&notes), LINE);
    assert_eq!(notes.len(), 1);
    assert_eq!(
        notes[0].why,
        vec![
            WhyWord::Evicted {
                model: model("qwen-3")
            },
            WhyWord::Warm
        ]
    );
    // The footer's `served` is as before: the route is additive.
    let view = w.companion.shared.answer(&opened.task).expect("answer");
    assert_eq!(view.footer.served, vec![served()]);
}

#[tokio::test]
async fn a_turn_that_announces_nothing_leaves_no_route() {
    let mut w = world(vec![words("Plain.")]);
    let opened = w.open("work").await;
    Box::pin(w.say(&opened.session, "hello")).await;
    // The scripted model sends only `Finished`: no stage, no route.
    assert_eq!(w.companion.shared.route_of(&opened.task), Some(vec![]));
}

#[tokio::test]
async fn a_named_model_that_cannot_serve_is_a_refusal_that_says_which_and_why() {
    let declined = Declined {
        model: model("kimi-k2.6"),
        because: DeclinedBecause::NotInstalled,
    };
    let mut w = world(vec![Say::Routed(
        vec![InferEvent::Declined(declined)],
        Box::new(Say::Refuse(InferRefusedError::Unavailable)),
    )]);
    let opened = w.open("work").await;
    Box::pin(w.say(&opened.session, "hello")).await;
    let view: AnswerWire = w.companion.shared.answer(&opened.task).expect("answer");
    assert_eq!(view.phase, AnswerPhase::Failed);
    assert_eq!(
        view.body,
        AnswerBody::Refused(RefusalWire::Failed(
            "Kimi K2.6 cannot answer: it is not installed on this computer.".into()
        ))
    );
}

#[tokio::test]
async fn no_model_chosen_asks_the_person_to_choose_one() {
    let mut w = world(vec![Say::Refuse(InferRefusedError::NoModelChosen)]);
    let opened = w.open("work").await;
    Box::pin(w.say(&opened.session, "hello")).await;
    let view: AnswerWire = w.companion.shared.answer(&opened.task).expect("answer");
    assert_eq!(view.phase, AnswerPhase::Failed);
    assert_eq!(view.body, AnswerBody::Refused(RefusalWire::NoModel));
}

#[tokio::test]
async fn automatic_with_no_model_that_uses_apps_says_so_in_plain_words() {
    let mut w = world(vec![Say::Refuse(InferRefusedError::NoModelUsesApps)]);
    let opened = w.open("work").await;
    Box::pin(w.say(&opened.session, "hello")).await;
    let view: AnswerWire = w.companion.shared.answer(&opened.task).expect("answer");
    assert_eq!(view.phase, AnswerPhase::Failed);
    assert_eq!(
        view.body,
        AnswerBody::Refused(RefusalWire::Failed(
            "None of the models in Automatic can use apps. Add one in Settings.".into()
        ))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_answer_object_serves_the_routing_property() {
    let scratch = tempfile::tempdir().expect("scratch");
    let bus = PrivateBus::start_in(scratch.path());
    let (server, client) = (bus.connect().await, bus.connect().await);
    let w = world(vec![Say::Routed(hosted(), Box::new(words("Done.")))]);
    let (launcher, hand) = (w.launcher, w.clock);
    let companion = Arc::new(Mutex::new(w.companion));
    let who = who(&server, scratch.path());
    serve_on_rooted(
        &server,
        Arc::clone(&companion),
        Arc::clone(&who),
        ProcRoot::Honoured(scratch.path().to_owned()),
    )
    .await
    .expect("serving");
    as_shell(&who, &client);
    let proxy = CompanionProxy::new(&client).await.expect("proxy");
    let mut added = proxy.receive_answer_added().await.expect("signal");
    let open = SessionOpen {
        space: space("work"),
        agent: AgentRef::Companion,
        parent: None,
        cwd: None,
        started_from: None,
        external: None,
        helper: None,
        acts_for: None,
    };
    let opened: SessionOpened = serde_json::from_str(
        &proxy
            .open(&serde_json::to_string(&open).expect("json"))
            .await
            .expect("open"),
    )
    .expect("opened");
    let turn = launcher
        .session_turn(
            opened.session.clone(),
            TurnIn {
                text: "hi".into(),
                origin: Origin::Launcher,
                keep: keep_nothing(),
                via: TurnVia::Typed,
                mode: None,
            },
        )
        .await
        .expect("turn");
    let ask = AskWire {
        session: opened.session.clone(),
        turn: UserTurn {
            id: turn,
            text: "hi".into(),
            at: hand.now(),
            from: Caller::launcher(),
            via: TurnVia::Typed,
            mode: None,
        },
        keep: keep_nothing(),
        parent_window: WindowKey::parse("w1").expect("window"),
        app: None,
    };
    let path = proxy
        .ask(&serde_json::to_string(&ask).expect("json"), &Details::new())
        .await
        .expect("ask");
    tokio::time::timeout(Duration::from_secs(2), added.next())
        .await
        .expect("AnswerAdded in time")
        .expect("signal");
    let answers = CompanionAnswerProxy::builder(&client)
        .path(path)
        .expect("path")
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await
        .expect("answer proxy");
    crate::serve::view_until_done(&answers).await;
    let notes: Vec<RouteNote> =
        serde_json::from_str(&answers.routing().await.expect("routing")).expect("notes");
    assert_eq!(footer_line(&notes), LINE);
}
