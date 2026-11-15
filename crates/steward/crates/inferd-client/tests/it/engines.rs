//! `EngineHost`: inference with no inferd, against porter's fake OpenAI-compatible server on
//! loopback (no network, no real model).
#![cfg(feature = "engines")]

use inferd_client::engines::{
    Dialect, Engine, EngineHost, EngineId, EngineUrl, KeySource, NoKeys, Route,
};
use inferd_client::{InferRefusal, InferSession, OpenOptions, Policy, SessionHost, Slot};
use porter_capability::{Locality, Tier};
use porter_core::AccountId;
use porter_core::{AppId, AppName, DataClass, Isolation, SecretText};
use porter_fake_servers::{FakeModels, ModelDef, ModelsHandle, Running, Wire};
use porter_ids::Tokens;
use porter_infer::{
    ChatControl, ChatMessage, ChatRequest, ClientFrame, InferEvent, InferReply, InferRequest,
    MessagePart, ModelError, Role, ToolParallelism,
};
use porter_match::LlmNeed;
use porter_match::Need;
use std::sync::{Arc, Mutex};

fn app() -> AppId {
    AppId {
        name: AppName::parse("org.example.Inapp").expect("name"),
        isolation: Isolation::Flatpak,
    }
}

fn need() -> Need {
    Need::Llm(LlmNeed::new(Default::default(), Tokens(1)))
}

fn chat(class: DataClass) -> InferRequest {
    InferRequest::Chat(
        ChatRequest::new(
            vec![ChatMessage {
                role: Role::User,
                parts: vec![MessagePart::Text("hello".into())],
            }],
            Tier::Balanced,
            class,
            porter_core::consent::Usage::Interactive,
        )
        .with_control(ChatControl::new().with_tool_calls(ToolParallelism::Many)),
    )
}

fn id(text: &str) -> EngineId {
    EngineId::parse(text).expect("id")
}

fn engine(name: &str, base: &str, locality: Locality) -> Engine {
    Engine::new(
        id(name),
        AccountId::parse(name).expect("account"),
        "llama3.2:3b",
        EngineUrl::parse(&format!("{base}/v1")).expect("url"),
        locality,
        Dialect::LlamaServer,
        Tokens(256),
    )
    .expect("engine")
}

fn cloud() -> Locality {
    Locality::Cloud { region: None }
}

fn row(engines: &[&str]) -> Route {
    Route {
        slot: Slot::Text,
        tier: Tier::Balanced,
        engines: engines.iter().map(|name| id(name)).collect(),
    }
}

/// Every class may go anywhere unless a floor says otherwise: what the app chose to say.
fn open_policy() -> Policy {
    Policy {
        local_only: porter_infer::LocalOnly::Off,
        floors: vec![],
    }
}

async fn fake(key: Option<&str>) -> Running<ModelsHandle> {
    FakeModels::start(Wire::OpenAi, vec![ModelDef::chat("llama3.2:3b", 8192)], key)
        .await
        .expect("fake")
}

async fn answer<S: InferSession>(session: &mut S, request: InferRequest) -> Vec<InferEvent> {
    session
        .send(ClientFrame::Request(request))
        .await
        .expect("send");
    let mut events = vec![];
    loop {
        let event = session.next().await.expect("event");
        let done = matches!(event, InferEvent::Finished(_));
        events.push(event);
        if done {
            return events;
        }
    }
}

fn finished(events: &[InferEvent]) -> &InferReply {
    match events.last() {
        Some(InferEvent::Finished(reply)) => reply,
        other => panic!("not finished: {other:?}"),
    }
}

async fn open<K: KeySource + 'static>(
    host: &EngineHost<K>,
    class: DataClass,
) -> inferd_client::engines::EngineSession<K> {
    host.open(
        &app(),
        &need(),
        class,
        Tier::Balanced,
        &OpenOptions::default(),
    )
    .await
    .expect("session")
}

#[tokio::test]
async fn a_chat_streams_its_events_from_the_local_engine() {
    let server = fake(None).await;
    server.say(&["Hel", "lo"]);
    let local = engine("ollama", server.base_url(), Locality::OnDevice);
    let host =
        EngineHost::new(vec![local], vec![row(&["ollama"])], open_policy(), NoKeys).expect("host");
    let mut session = open(&host, DataClass::Mail).await;
    let events = answer(&mut session, chat(DataClass::Mail)).await;

    assert!(matches!(&events[0], InferEvent::Routed(served)
        if served.account.as_str() == "ollama"
            && served.model.as_str() == "llama3.2-3b"
            && served.locality == Locality::OnDevice));
    let deltas: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            InferEvent::TextDelta(text) => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(deltas, vec!["Hel", "lo"]);
    match finished(&events) {
        InferReply::Chat(reply) => assert_eq!(reply.text, "Hello"),
        other => panic!("{other:?}"),
    }
    // The request carried the served name and the engine's reply limit, and no temperature:
    // the app chose no sampling, so it is the engine's.
    let sent = &server.chats()[0];
    assert_eq!(sent["model"], "llama3.2:3b");
    assert_eq!(sent["max_tokens"], 256);
    assert!(sent.get("temperature").is_none(), "{sent}");
    assert_eq!(server.hits()[0].authorization, None);

    // A second request on the same session is a second turn; Routed is not repeated.
    let again = answer(&mut session, chat(DataClass::Mail)).await;
    assert!(!again.iter().any(|e| matches!(e, InferEvent::Routed(_))));
    assert_eq!(server.chats().len(), 2);
}

/// A key source holding one key per account, and counting how often it was asked.
#[derive(Debug, Default)]
struct Keychain {
    keys: Mutex<Vec<(String, String)>>,
}

impl KeySource for Keychain {
    async fn key(&self, account: &AccountId) -> Option<SecretText> {
        self.keys
            .lock()
            .expect("lock")
            .iter()
            .find(|(name, _)| name == account.as_str())
            .map(|(_, key)| SecretText::new(key.clone()))
    }
}

#[tokio::test]
async fn a_missing_key_is_a_typed_refusal_and_a_held_one_goes_as_the_bearer() {
    let server = fake(Some("sk-fake")).await;
    let hosted = engine("hosted", server.base_url(), cloud()).with_key();
    let keychain = Arc::new(Keychain::default());
    let host = EngineHost::new(
        vec![hosted],
        vec![row(&["hosted"])],
        open_policy(),
        Arc::clone(&keychain),
    )
    .expect("host");

    let mut session = open(&host, DataClass::Public).await;
    let events = answer(&mut session, chat(DataClass::Public)).await;
    assert_eq!(
        finished(&events),
        &InferReply::Refused(InferRefusal::NeedsGrant)
    );
    assert!(server.hits().is_empty());

    keychain
        .keys
        .lock()
        .expect("lock")
        .push(("hosted".into(), "sk-fake".into()));
    let mut session = open(&host, DataClass::Public).await;
    let events = answer(&mut session, chat(DataClass::Public)).await;
    assert!(matches!(finished(&events), InferReply::Chat(_)));
    assert_eq!(
        server.hits()[0].authorization.as_deref(),
        Some("Bearer sk-fake")
    );
}

#[tokio::test]
async fn an_engine_that_is_not_running_is_a_failed_turn_not_a_hang() {
    // A port nobody listens on and nothing else can take: a socket bound to it and not
    // listening, so a connect is refused. (A port freed by dropping a listener can be handed to
    // another test's fake before the connect.)
    let reserved = tokio::net::TcpSocket::new_v4().expect("socket");
    reserved
        .bind(std::net::SocketAddr::from(([127, 0, 0, 1], 0)))
        .expect("bind an ephemeral port");
    let port = reserved.local_addr().expect("addr").port();
    let local = engine(
        "ollama",
        &format!("http://127.0.0.1:{port}"),
        Locality::OnDevice,
    );
    let host =
        EngineHost::new(vec![local], vec![row(&["ollama"])], open_policy(), NoKeys).expect("host");
    let mut session = open(&host, DataClass::Public).await;
    let events = answer(&mut session, chat(DataClass::Public)).await;
    assert_eq!(
        finished(&events),
        &InferReply::Failed(ModelError::Unreachable)
    );
}

#[tokio::test]
async fn prepare_says_what_routing_says() {
    let server = fake(None).await;
    let local = engine("ollama", server.base_url(), Locality::OnDevice);
    let host =
        EngineHost::new(vec![local], vec![row(&["ollama"])], open_policy(), NoKeys).expect("host");
    let options = OpenOptions::default();
    assert_eq!(
        host.prepare(&app(), &need(), DataClass::Public, Tier::Balanced, &options)
            .await,
        Ok(porter_infer::Readiness::Ready)
    );
    // No row for the Best tier: nothing chosen, said as a refusal naming it.
    assert!(matches!(
        host.prepare(&app(), &need(), DataClass::Public, Tier::Best, &options)
            .await,
        Err(inferd_client::TransportError::Denied(why)) if why.contains("no model is chosen")
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_cancelled_turn_never_reaches_the_next_one() {
    let server = fake(None).await;
    let local = engine("ollama", server.base_url(), Locality::OnDevice);
    let host =
        EngineHost::new(vec![local], vec![row(&["ollama"])], open_policy(), NoKeys).expect("host");
    let mut session = open(&host, DataClass::Public).await;
    let old = vec!["old"; 400];
    // The race is a cancelled task still sending as the next turn starts, so it is tried many
    // times, on several threads. Without a channel per turn, an "old" delta (or the old turn's
    // Finished) lands among the new turn's events.
    for round in 0..60 {
        server.say(&old);
        session
            .send(ClientFrame::Request(chat(DataClass::Public)))
            .await
            .expect("send");
        loop {
            match session.next().await.expect("event") {
                InferEvent::TextDelta(_) => break,
                InferEvent::Routed(_) => {}
                other => panic!("round {round}: {other:?}"),
            }
        }
        session.send(ClientFrame::Cancel).await.expect("cancel");
        server.say(&["new"]);
        session
            .send(ClientFrame::Request(chat(DataClass::Public)))
            .await
            .expect("send");

        let mut cancelled = 0;
        let last = loop {
            match session.next().await.expect("event") {
                InferEvent::TextDelta(text) => {
                    assert_eq!(text, "new", "round {round}: the old turn leaked");
                }
                InferEvent::Finished(InferReply::Cancelled) => cancelled += 1,
                InferEvent::Finished(reply) => break reply,
                other => panic!("round {round}: {other:?}"),
            }
        };
        assert_eq!(cancelled, 1, "round {round}");
        match last {
            InferReply::Chat(reply) => assert_eq!(reply.text, "new", "round {round}"),
            other => panic!("round {round}: {other:?}"),
        }
        // Nothing else follows: the session waits for a request.
        let more = tokio::time::timeout(std::time::Duration::from_millis(5), session.next()).await;
        assert!(more.is_err(), "round {round}: a stray event {more:?}");
    }
}
