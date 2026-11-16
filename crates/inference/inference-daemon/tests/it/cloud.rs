//! Hosted models, end to end on a private bus: the real `Inference1` object, a fake accountd that
//! answers `Verdicts` and hands keys on sealed memfds, and a fake provider that speaks the
//! OpenAI-compatible chat API over TLS on loopback (the scratch CA, trusted through `Roots::Only`).
//! Acceptance 7 of design/31 section 7.1: no key is on any bus, in any file or in any audit line,
//! checked by a scan that has a positive control.

use crate::hosting;

use account_capability::LlmFeature;
use account_capability::Tier;
use account_core::consent::Usage;
use account_core::{AppId, AppName, DataClass, Isolation};
use account_match::LlmNeed;
use account_match::Need;
use base_ids::{Tokens, TurnCount};
use hosting::accountd::{FakeAccount, Standing};
use hosting::bus::within;
use hosting::engine::{Chat, Script};
use hosting::entries;
use hosting::rig::{Hosted, Plan, Trust, World};
use inference_client::InferSession;
use inference_core::{
    ChargeTotal, ChatMessage, ChatRequest, ClientFrame, InferEvent, InferRefusal, InferReply,
    InferRequest, MessagePart, ModelError, OpenOptions, Policy, Role as ChatRole, StopReason,
};
use inference_dbus::inference::InferenceProxy;
use model_provider::Money;
use rust_decimal::Decimal;
use std::path::Path;

/// `OpenRouter`'s key, and `OpenAI`'s: distinctive, so a scan for either cannot match by accident.
const KEY_OR: &str = "sk-or-v1-S3CRET-CLOUD-KEY-0123456789";
const KEY_OAI: &str = "sk-proj-OTHER-SECRET-KEY-9876543210";

pub(crate) const COMPANION: &str = "org.bardo.Companion";
const READER: &str = "org.bardo.Reader";

pub(crate) fn app(name: &str) -> AppId {
    AppId {
        name: AppName::parse(name).expect("name"),
        isolation: Isolation::Unsandboxed,
    }
}

fn llm() -> Need {
    Need::Llm(LlmNeed::new([LlmFeature::Chat].into(), Tokens(1000)))
}

fn chat_request(text: &str, class: DataClass) -> InferRequest {
    InferRequest::Chat(ChatRequest::new(
        vec![ChatMessage {
            role: ChatRole::User,
            parts: vec![MessagePart::Text(text.into())],
        }],
        Tier::Balanced,
        class,
        Usage::Interactive,
    ))
}

async fn until_finished(session: &mut impl InferSession) -> Vec<InferEvent> {
    let mut events = Vec::new();
    loop {
        let event = within(
            &format!("the session's next event after {events:?}"),
            session.next(),
        )
        .await
        .expect("an event");
        let done = matches!(event, InferEvent::Finished(_));
        events.push(event);
        if done {
            return events;
        }
    }
}

/// One chat turn of the world's app, the events it produced.
pub(crate) async fn turn(world: &World, text: &str) -> Vec<InferEvent> {
    let mut session = within(
        "inferd's answer to Open",
        world
            .accounts
            .session(&llm(), DataClass::Prompt, Tier::Balanced),
    )
    .await
    .expect("open");
    // A refused session closes by itself: the send may find it already gone, and its one event
    // is still waiting to be read.
    let _ = within(
        "the session to take the request",
        session.send(ClientFrame::Request(chat_request(text, DataClass::Prompt))),
    )
    .await;
    until_finished(&mut session).await
}

pub(crate) fn finished(events: &[InferEvent]) -> &InferReply {
    match events.last() {
        Some(InferEvent::Finished(reply)) => reply,
        other => panic!("a finished session, got {other:?}"),
    }
}

/// One chat turn of a session opened as `usage` (the option of its `Open`), the events it produced.
async fn turn_as(world: &World, usage: Usage) -> Vec<InferEvent> {
    let mut session = world
        .accounts
        .session_with(
            &llm(),
            DataClass::Prompt,
            Tier::Balanced,
            &OpenOptions::default().with_usage(usage),
        )
        .await
        .expect("open");
    let mut request = chat_request("indexing", DataClass::Prompt);
    if let InferRequest::Chat(chat) = &mut request {
        chat.usage = usage;
    }
    let _ = session.send(ClientFrame::Request(request)).await;
    until_finished(&mut session).await
}

fn grant_for(usage: &'static str) -> FakeAccount {
    FakeAccount {
        id: "openrouter",
        standing: Standing::GrantedFor {
            usage,
            to: vec![COMPANION],
            grant: "grant-openrouter",
            key: Some(KEY_OR),
        },
    }
}

/// A person who allows cloud models for every class: the floors are lowered and local-only is off.
pub(crate) fn open_policy() -> Policy {
    Policy {
        local_only: false,
        floors: Vec::new(),
    }
}

fn openrouter(to: &[&'static str]) -> FakeAccount {
    FakeAccount {
        id: "openrouter",
        standing: Standing::Granted {
            to: to.to_vec(),
            grant: "grant-openrouter",
            key: Some(KEY_OR),
        },
    }
}

fn openai(to: &[&'static str]) -> FakeAccount {
    FakeAccount {
        id: "openai",
        standing: Standing::Granted {
            to: to.to_vec(),
            grant: "grant-openai",
            key: Some(KEY_OAI),
        },
    }
}

fn plan(accounts: Vec<FakeAccount>, app_name: &str) -> Plan {
    Plan {
        catalog: vec![
            // Listed first: with no price to compare, Automatic takes the first of equals.
            ("a-gpt-6-luna.toml", entries::luna()),
            ("b-claude-opus-5.5.toml", entries::claude()),
            ("c-kimi-k3.toml", entries::kimi()),
        ],
        policy: open_policy(),
        app: Some(app(app_name)),
        hosted: Some(Hosted {
            accounts,
            chat: Script {
                chat: vec![Chat::Say(vec!["Hel", "lo"])],
                dims: 0,
            },
            trust: Trust::ScratchCa,
        }),
        ..Plan::default()
    }
}

pub(crate) fn routed(events: &[InferEvent]) -> &inference_core::ServedBy {
    events
        .iter()
        .find_map(|event| match event {
            InferEvent::Routed(served) => Some(served),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a Routed event in {events:?}"))
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

/// Every message on the bus from the moment the tap is set: the bytes as a monitor sees them.
struct Tap(zbus::MessageStream);

impl Tap {
    async fn start(bus: &hosting::bus::PrivateBus) -> Self {
        let monitor = bus.connect().await;
        zbus::fdo::MonitoringProxy::new(&monitor)
            .await
            .expect("proxy")
            .become_monitor(&[], 0)
            .await
            .expect("monitor");
        Self(zbus::MessageStream::from(monitor))
    }

    async fn drain(&mut self) -> Vec<Vec<u8>> {
        use zbus::export::futures_core::Stream;
        let mut seen = Vec::new();
        loop {
            let next = tokio::time::timeout(
                std::time::Duration::from_millis(300),
                std::future::poll_fn(|cx| std::pin::Pin::new(&mut self.0).poll_next(cx)),
            )
            .await;
            match next {
                Ok(Some(Ok(message))) => seen.push(message.data().to_vec()),
                _ => return seen,
            }
        }
    }
}

/// Every file under `dir` that holds `needle`.
fn files_holding(dir: &Path, needle: &str) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            found.extend(files_holding(&path, needle));
        } else if kind.is_file() && std::fs::read(&path).is_ok_and(|bytes| contains(&bytes, needle))
        {
            found.push(path);
        }
    }
    found
}

#[tokio::test(flavor = "multi_thread")]
async fn acceptance_7_a_chat_turn_goes_to_a_hosted_model_through_openrouter_and_no_key_is_on_the_bus()
 {
    let world = World::start(plan(vec![openrouter(&[COMPANION])], COMPANION)).await;
    let mut tap = Tap::start(&world.bus).await;

    let events = turn(&world, "hi there").await;

    // Routed to a hosted entry through the account of the grant, among the entries.
    let served = routed(&events);
    assert_eq!(served.account.as_str(), "openrouter");
    assert_eq!(served.model.as_str(), "gpt-6-luna");
    assert!(matches!(
        served.locality,
        account_capability::Locality::Cloud { .. }
    ));
    // The Answer note names the hosted entry by its label, after Routed and before any token.
    let at = |want: fn(&InferEvent) -> bool| events.iter().position(want).expect("event");
    let (routed_at, stage_at, first_token) = (
        at(|e| matches!(e, InferEvent::Routed(_))),
        at(|e| matches!(e, InferEvent::Stage(_))),
        at(|e| matches!(e, InferEvent::TextDelta(_))),
    );
    assert!(routed_at < stage_at && stage_at < first_token, "{events:?}");
    let InferEvent::Stage(note) = &events[stage_at] else {
        unreachable!()
    };
    assert_eq!(note.role, inference_core::StageRole::Answer);
    assert_eq!(
        note.name,
        Some(inference_core::ModelLabel("GPT-6 Luna".into()))
    );
    let InferReply::Chat(reply) = finished(&events) else {
        panic!("a chat reply, got {events:?}");
    };
    assert_eq!(reply.text, "Hello");
    assert_eq!(reply.stop, StopReason::EndTurn);
    assert_eq!(
        (reply.usage.input, reply.usage.output),
        (Tokens(5), Tokens(2))
    );

    // What the provider saw: the key in Authorization, OpenRouter's name for the model, the base
    // path of the provider, and no temperature (the app chose none).
    let provider = world.provider.as_ref().expect("provider");
    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/api/v1/chat/completions");
    assert_eq!(
        requests[0].header("authorization"),
        Some(format!("Bearer {KEY_OR}").as_str())
    );
    assert_eq!(requests[0].body["model"], "openai/gpt-6-luna");
    assert!(
        requests[0].body.get("temperature").is_none(),
        "{}",
        requests[0].body
    );
    assert!(requests[0].body.to_string().contains("hi there"));

    // What accountd was asked: who the app is, and the one grant the key was resolved under.
    let calls = world.accountd.as_ref().expect("accountd").calls();
    assert_eq!(
        calls.verdicts,
        vec![(COMPANION.to_owned(), "prompt".to_owned())]
    );
    assert_eq!(calls.resolved, vec!["grant-openrouter".to_owned()]);

    // The turn was audited against the account that served it, and metered at what OpenRouter
    // reported for it: five tokens in, two out, and a charge of 0.00095 dollars.
    let audited = world.audit.entries();
    assert_eq!(audited.len(), 1);
    assert_eq!(audited[0].account.as_str(), "openrouter");
    assert!(matches!(
        audited[0].locality,
        account_capability::Locality::Cloud { .. }
    ));
    assert_eq!(audited[0].usage.charge, Some(reported()));
    assert_eq!(reply.usage.charge, Some(reported()));
    let cloud = world.served.cloud().expect("cloud");
    let spent = cloud.ledger().spent(
        &inference_core::SpendScope::App(app(COMPANION)),
        inference_core::Period::Daily,
        cloud.now(),
    );
    assert_eq!(
        (spent.charges, spent.tokens_in, spent.tokens_out),
        (
            ChargeTotal {
                reported: reported(),
                unpriced: TurnCount(0)
            },
            5,
            2
        )
    );

    // Acceptance 7. The positive control first: a key sent as an argument on the bus is found by
    // the scan (a stranger sending the key as the name of a computer to remove: refused, but the
    // call crossed the bus). The key itself is read in this process and never leaves it.
    let stranger = world.bus.connect().await;
    let _ = InferenceProxy::new(&stranger)
        .await
        .expect("proxy")
        .remove_computer(KEY_OR)
        .await;
    let seen = tap.drain().await;
    assert!(
        seen.len() > 2,
        "the monitor saw the traffic: {}",
        seen.len()
    );
    let carrying: Vec<_> = seen.iter().filter(|m| contains(m, KEY_OR)).collect();
    assert_eq!(
        carrying.len(),
        1,
        "only the deliberate control carries the key"
    );
    assert!(
        carrying[0].windows(14).any(|w| w == b"RemoveComputer"),
        "and it is the control's call"
    );
    // Nor is it in an audit line, a file the daemon's world holds, or the engines' debug text.
    let lines = serde_json::to_string(&audited).expect("json");
    assert!(!lines.contains(KEY_OR), "{lines}");
    assert_eq!(
        files_holding(&world.scratch, KEY_OR),
        Vec::<std::path::PathBuf>::new()
    );
    assert!(!format!("{:?}", world.served).contains(KEY_OR));
}

#[tokio::test(flavor = "multi_thread")]
async fn each_turn_fetches_the_key_again_and_a_provider_gets_only_that_turns_copy() {
    let world = World::start(plan(vec![openrouter(&[COMPANION])], COMPANION)).await;
    turn(&world, "one").await;
    turn(&world, "two").await;
    let calls = world.accountd.as_ref().expect("accountd").calls();
    assert_eq!(calls.resolved.len(), 2, "{calls:?}");
    let requests = world.provider.as_ref().expect("provider").requests();
    assert_eq!(requests.len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_direct_grant_beats_openrouter_and_the_key_is_that_accounts() {
    let accounts = vec![openrouter(&[COMPANION]), openai(&[COMPANION])];
    let world = World::start(plan(accounts, COMPANION)).await;
    let events = turn(&world, "hi").await;
    let served = routed(&events);
    assert_eq!(
        served.account.as_str(),
        "openai",
        "the company's own account"
    );
    assert_eq!(served.model.as_str(), "gpt-6-luna");
    let requests = world.provider.as_ref().expect("provider").requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/v1/chat/completions");
    assert_eq!(
        requests[0].header("authorization"),
        Some(format!("Bearer {KEY_OAI}").as_str())
    );
    assert_eq!(
        requests[0].body["model"], "gpt-6-luna",
        "OpenAI's own name for it"
    );
    let calls = world.accountd.as_ref().expect("accountd").calls();
    assert_eq!(
        calls.resolved,
        vec!["grant-openai".to_owned()],
        "the other key was never asked for"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_app_without_a_grant_is_never_served_a_cloud_model_even_with_the_floor_lowered() {
    // The Reader holds no grant; the Companion does. The floor is lowered and local-only is off.
    let world = World::start(plan(vec![openrouter(&[COMPANION])], READER)).await;
    let events = turn(&world, "secret").await;
    assert_eq!(
        events,
        vec![InferEvent::Finished(InferReply::Refused(
            InferRefusal::NeedsGrant
        ))]
    );
    assert!(
        world
            .provider
            .as_ref()
            .expect("provider")
            .requests()
            .is_empty()
    );
    let calls = world.accountd.as_ref().expect("accountd").calls();
    assert_eq!(
        calls.verdicts,
        vec![(READER.to_owned(), "prompt".to_owned())]
    );
    assert_eq!(calls.resolved, Vec::<String>::new(), "no key was fetched");
    assert!(world.audit.entries().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_account_the_person_denied_the_app_is_a_refusal_and_no_account_is_unavailable() {
    let denied = FakeAccount {
        id: "openrouter",
        standing: Standing::Denied,
    };
    let world = World::start(plan(vec![denied], COMPANION)).await;
    assert_eq!(
        turn(&world, "x").await,
        vec![InferEvent::Finished(InferReply::Refused(
            InferRefusal::Denied
        ))]
    );
    let none = World::start(plan(Vec::new(), COMPANION)).await;
    assert_eq!(
        turn(&none, "x").await,
        vec![InferEvent::Finished(InferReply::Refused(
            InferRefusal::Unavailable
        ))]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_key_accountd_will_not_give_is_unauthorized_and_nothing_is_sent() {
    let refused = FakeAccount {
        id: "openrouter",
        standing: Standing::Granted {
            to: vec![COMPANION],
            grant: "grant-openrouter",
            key: None,
        },
    };
    let world = World::start(plan(vec![refused], COMPANION)).await;
    let events = turn(&world, "x").await;
    assert_eq!(
        finished(&events),
        &InferReply::Failed(ModelError::Unauthorized),
        "{events:?}"
    );
    assert!(
        world
            .provider
            .as_ref()
            .expect("provider")
            .requests()
            .is_empty()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_server_the_roots_do_not_vouch_for_gets_no_request_and_no_key() {
    let mut world_plan = plan(vec![openrouter(&[COMPANION])], COMPANION);
    if let Some(hosted) = world_plan.hosted.as_mut() {
        hosted.trust = Trust::NoOne;
    }
    let world = World::start(world_plan).await;
    let events = turn(&world, "x").await;
    assert_eq!(
        finished(&events),
        &InferReply::Failed(ModelError::Unreachable),
        "{events:?}"
    );
    assert!(
        world
            .provider
            .as_ref()
            .expect("provider")
            .requests()
            .is_empty(),
        "the handshake failed before any request was written"
    );
    assert_eq!(
        world.audit.entries().len(),
        1,
        "a failed turn is still audited, without usage"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_provider_that_rejects_the_key_is_unauthorized_and_charges_nothing() {
    let world = failing_with(401).await;
    let events = turn(&world, "x").await;
    assert_eq!(
        finished(&events),
        &InferReply::Failed(ModelError::Unauthorized),
        "{events:?}"
    );
    // The provider says no model ran, so there is no turn to count.
    assert_eq!(spent_today(&world).charges, ChargeTotal::default());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_server_that_answers_an_error_status_charges_nothing_for_any_attempt() {
    let world = failing_with(500).await;
    let events = turn(&world, "x").await;
    assert!(
        matches!(finished(&events), InferReply::Failed(_)),
        "{events:?}"
    );
    // An error status before any reply is a request no model ran, retries included; a stream
    // that breaks after the model started is counted as an unpriced turn (`Unknown`).
    assert_eq!(spent_today(&world).charges, ChargeTotal::default());
}

/// A world whose hosted provider answers every turn with `status`.
async fn failing_with(status: u16) -> World {
    let mut world_plan = plan(vec![openrouter(&[COMPANION])], COMPANION);
    if let Some(hosted) = world_plan.hosted.as_mut() {
        hosted.chat = Script {
            chat: vec![Chat::Fail(status); 4],
            dims: 0,
        };
    }
    World::start(world_plan).await
}

/// What `OpenRouter` reports for the scripted reply: 0.00095 US dollars.
fn reported() -> Money {
    Money::usd(Decimal::new(95, 5))
}

/// What the companion has spent today.
fn spent_today(world: &World) -> inference_daemon::cloud::spend::Spent {
    let cloud = world.served.cloud().expect("cloud");
    cloud.ledger().spent(
        &inference_core::SpendScope::App(app(COMPANION)),
        inference_core::Period::Daily,
        cloud.now(),
    )
}

fn tools_request() -> InferRequest {
    let InferRequest::Chat(chat) = chat_request("look it up", DataClass::Prompt) else {
        unreachable!("a chat request");
    };
    InferRequest::Chat(chat.with_tools(vec![inference_core::ToolDecl {
        name: inference_core::ToolName::new("lookup").expect("name"),
        description: "look something up".into(),
        params: inference_core::SchemaText(
            inference_core::JsonText::new(r#"{"type":"object","properties":{}}"#).expect("json"),
        ),
    }]))
}

#[tokio::test(flavor = "multi_thread")]
async fn luna_with_tools_is_told_to_reason_off_in_the_provider_spelling_and_without_tools_it_is_not()
 {
    for (accounts, field, want) in [
        (
            vec![openai(&[COMPANION])],
            "reasoning_effort",
            serde_json::json!("none"),
        ),
        (
            vec![openrouter(&[COMPANION])],
            "reasoning",
            serde_json::json!({"effort": "none"}),
        ),
    ] {
        let world = World::start(plan(accounts, COMPANION)).await;
        let mut session = world
            .accounts
            .session(&llm(), DataClass::Prompt, Tier::Balanced)
            .await
            .expect("open");
        session
            .send(ClientFrame::Request(tools_request()))
            .await
            .expect("send");
        until_finished(&mut session).await;
        session
            .send(ClientFrame::Request(chat_request(
                "plain",
                DataClass::Prompt,
            )))
            .await
            .expect("send");
        until_finished(&mut session).await;
        let requests = world.provider.as_ref().expect("provider").requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].body[field], want, "{}", requests[0].body);
        assert!(
            requests[1].body.get(field).is_none(),
            "{}",
            requests[1].body
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_session_asks_accountd_for_the_usage_it_was_opened_with() {
    let world = World::start(plan(vec![openrouter(&[COMPANION])], COMPANION)).await;
    for usage in [Usage::Background, Usage::Interactive] {
        let offered = world
            .served
            .offer(&app(COMPANION), DataClass::Prompt, usage)
            .await;
        assert_eq!(offered.models().count(), 3, "{usage:?}");
    }
    let calls = world.accountd.as_ref().expect("accountd").calls();
    assert_eq!(
        calls.usages,
        vec!["background".to_owned(), "interactive".to_owned()]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_background_session_asks_verdicts_as_background_and_an_interactive_one_as_interactive() {
    let world = World::start(plan(vec![grant_for("background")], COMPANION)).await;
    let background = turn_as(&world, Usage::Background).await;
    assert_eq!(routed(&background).account.as_str(), "openrouter");
    let interactive = turn(&world, "hi").await;
    assert_eq!(
        interactive,
        vec![InferEvent::Finished(InferReply::Refused(
            InferRefusal::NeedsGrant
        ))]
    );
    let calls = world.accountd.as_ref().expect("accountd").calls();
    assert_eq!(calls.usages, vec!["background", "interactive"], "{calls:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_background_session_is_not_served_on_a_grant_held_for_interactive_use_only() {
    let world = World::start(plan(vec![grant_for("interactive")], COMPANION)).await;
    assert_eq!(
        turn_as(&world, Usage::Background).await,
        vec![InferEvent::Finished(InferReply::Refused(
            InferRefusal::NeedsGrant
        ))]
    );
    assert!(
        world
            .provider
            .as_ref()
            .expect("provider")
            .requests()
            .is_empty()
    );
    let calls = world.accountd.as_ref().expect("accountd").calls();
    assert_eq!(calls.usages, vec!["background"]);
    assert_eq!(calls.resolved, Vec::<String>::new(), "no key was fetched");
    // The same grant serves a session that names no usage.
    assert_eq!(
        routed(&turn(&world, "hi").await).account.as_str(),
        "openrouter"
    );
}
