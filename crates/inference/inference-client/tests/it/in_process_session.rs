//! `Brokered` and inference: an app that hosts no inference has no inferd (a session is
//! `Unreachable`, as on a bus with no daemon, so a caller degrades); one that hands in a broker
//! gets the broker's sessions, opened for the app the host names.

use account_capability::Tier;
use account_core::{AppId, AppName, DataClass, Isolation};
use account_match::LlmNeed;
use account_match::Need;
use base_ids::Tokens;
use inference_client::{
    Brokered, ClientError, InferSession, Inference, NoBroker, SessionHost, TransportError,
};
use inference_core::testing::{FakeInferSession, Script, ScriptStep};
use inference_core::{
    ChatControl, ChatReply, ClientFrame, InferEvent, InferReply, InferRequest, OpenOptions,
    RequestKind, ServedBy, StopReason, TokenUsage, ToolParallelism, Traceparent,
};
use std::sync::{Arc, Mutex};

fn app() -> AppId {
    AppId {
        name: AppName::parse("org.bardo.Mail").expect("name"),
        isolation: Isolation::Flatpak,
    }
}

fn need() -> Need {
    Need::Llm(LlmNeed::new(
        std::collections::BTreeSet::default(),
        Tokens(1),
    ))
}

fn served() -> ServedBy {
    ServedBy {
        account: account_core::AccountId::parse("local").expect("id"),
        model: account_capability::ModelId::parse("echo").expect("id"),
        locality: account_capability::Locality::OnDevice,
    }
}

fn chat() -> InferRequest {
    InferRequest::Chat(
        inference_core::ChatRequest::new(
            vec![],
            Tier::Fast,
            DataClass::Public,
            account_core::consent::Usage::Interactive,
        )
        .with_control(ChatControl::new().with_tool_calls(ToolParallelism::Many)),
    )
}

fn reply() -> InferReply {
    InferReply::Chat(ChatReply::new(
        "hello".into(),
        StopReason::EndTurn,
        TokenUsage {
            input: Tokens(1),
            output: Tokens(1),
            cached: Tokens(0),
            charge: None,
        },
        served(),
    ))
}

#[tokio::test]
async fn with_no_broker_a_session_is_unreachable() {
    let accounts = Inference::over(Brokered::new(app(), NoBroker));
    assert!(matches!(
        accounts
            .session(&need(), DataClass::Public, Tier::Fast)
            .await,
        Err(ClientError::Transport(TransportError::Unreachable))
    ));
    assert_eq!(
        accounts
            .infer(&need(), DataClass::Public, Tier::Fast, chat())
            .await,
        Err(ClientError::Transport(TransportError::Unreachable))
    );
}

#[tokio::test]
async fn the_guest_and_computer_calls_are_unsupported_when_hosted() {
    let accounts = Inference::over(Brokered::new(app(), NoBroker));
    let node = account_tailscale::NodeId::parse("nPI").expect("node");
    let unsupported = Some(ClientError::Transport(TransportError::Unsupported));
    assert_eq!(accounts.guests().await.err(), unsupported);
    assert_eq!(accounts.candidates().await.err(), unsupported);
    assert_eq!(
        accounts
            .answer_guest(&node, inference_tailnet::GuestAnswer::Allow)
            .await
            .err(),
        unsupported
    );
    assert_eq!(accounts.forget_guest(&node).await.err(), unsupported);
    assert_eq!(
        accounts.add_tailnet_computer(&node).await.err(),
        unsupported
    );
}

/// What the host was asked to open.
type Opened = Vec<(AppId, DataClass, Tier, OpenOptions)>;

#[derive(Debug, Default)]
struct Broker {
    opened: Mutex<Opened>,
}

impl SessionHost for Broker {
    type Session = FakeInferSession;

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn open(
        &self,
        app: &AppId,
        _need: &Need,
        class: DataClass,
        tier: Tier,
        options: &OpenOptions,
    ) -> Result<FakeInferSession, TransportError> {
        self.opened
            .lock()
            .expect("lock")
            .push((app.clone(), class, tier, options.clone()));
        Ok(FakeInferSession::scripted([Script {
            kind: RequestKind::Chat,
            steps: vec![
                ScriptStep::Emit(InferEvent::Routed(served())),
                ScriptStep::Emit(InferEvent::Finished(reply())),
            ],
        }]))
    }
}

#[tokio::test]
async fn a_hosted_broker_serves_the_sessions_for_the_app_the_host_names() {
    let broker = Arc::new(Broker::default());
    let accounts = Inference::over(Brokered::new(app(), Arc::clone(&broker)));
    let parent = Traceparent::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01")
        .expect("traceparent");
    let options = OpenOptions::default().with_traceparent(parent);
    let mut session = accounts
        .session_with(&need(), DataClass::Public, Tier::Fast, &options)
        .await
        .expect("a session");
    session
        .send(ClientFrame::Request(chat()))
        .await
        .expect("send");
    assert_eq!(session.next().await, Ok(InferEvent::Routed(served())));
    assert_eq!(session.next().await, Ok(InferEvent::Finished(reply())));
    assert_eq!(
        *broker.opened.lock().expect("lock"),
        vec![(app(), DataClass::Public, Tier::Fast, options)]
    );

    // The one-call form goes through the same door.
    assert_eq!(
        accounts
            .infer(&need(), DataClass::Public, Tier::Fast, chat())
            .await,
        Ok(reply())
    );
}
