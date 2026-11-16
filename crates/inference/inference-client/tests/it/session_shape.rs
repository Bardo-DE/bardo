//! `Inference::session` and `Inference::infer` over a transport whose sessions are scripted.

use inference_client::{ClientError, InferSession, Inference, Transport, TransportError};
use account_capability::Tier;
use account_core::DataClass;
use account_ids::Tokens;
use inference_core::OpenOptions;
use inference_core::testing::{FakeInferSession, Script, ScriptStep};
use inference_core::{ChatControl, ToolParallelism};
use inference_core::{
    ChatReply, ClientFrame, InferEvent, InferRefusal, InferReply, InferRequest, RequestKind,
    ServedBy, StopReason, TokenUsage,
};
use account_match::LlmNeed;
use account_match::Need;
use std::sync::Mutex;

#[derive(Debug)]
struct Scripted(Mutex<Option<FakeInferSession>>);

impl Transport for Scripted {
    type Session = FakeInferSession;

    async fn open_with(
        &self,
        _need: &Need,
        _class: DataClass,
        _tier: Tier,
        _options: &OpenOptions,
    ) -> Result<FakeInferSession, TransportError> {
        self.0
            .lock()
            .expect("lock")
            .take()
            .ok_or(TransportError::Unreachable)
    }
}

fn need() -> Need {
    Need::Llm(LlmNeed::new(Default::default(), Tokens(1)))
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

fn accounts(script: Script) -> Inference<Scripted> {
    Inference::over(Scripted(Mutex::new(Some(FakeInferSession::scripted([
        script,
    ])))))
}

#[tokio::test]
async fn infer_reads_to_the_finished_event() {
    let usage = TokenUsage {
        input: Tokens(1),
        output: Tokens(1),
        cached: Tokens(0),
        charge: None,
    };
    let reply = InferReply::Chat(ChatReply::new(
        "hi".into(),
        StopReason::EndTurn,
        usage,
        served(),
    ));
    let script = Script {
        kind: RequestKind::Chat,
        steps: vec![
            ScriptStep::Emit(InferEvent::Routed(served())),
            ScriptStep::Emit(InferEvent::TextDelta("hi".into())),
            ScriptStep::Emit(InferEvent::Finished(reply.clone())),
        ],
    };
    let got = accounts(script)
        .infer(&need(), DataClass::Public, Tier::Fast, chat())
        .await;
    assert_eq!(got, Ok(reply));
}

#[tokio::test]
async fn a_refusal_is_an_error_the_app_shows() {
    let script = Script {
        kind: RequestKind::Chat,
        steps: vec![ScriptStep::Emit(InferEvent::Finished(InferReply::Refused(
            InferRefusal::NeedsGrant,
        )))],
    };
    let got = accounts(script)
        .infer(&need(), DataClass::Mail, Tier::Fast, chat())
        .await;
    assert_eq!(
        got,
        Err(ClientError::InferRefused(InferRefusal::NeedsGrant))
    );
}

#[tokio::test]
async fn a_session_streams_its_events_one_by_one() {
    let script = Script {
        kind: RequestKind::Chat,
        steps: vec![ScriptStep::Emit(InferEvent::TextDelta("a".into()))],
    };
    let mut session = accounts(script)
        .session(&need(), DataClass::Public, Tier::Fast)
        .await
        .expect("session");
    session
        .send(ClientFrame::Request(chat()))
        .await
        .expect("send");
    assert_eq!(session.next().await, Ok(InferEvent::TextDelta("a".into())));
}

#[cfg(unix)]
#[tokio::test]
async fn a_session_may_take_attachments_with_a_frame() {
    let mut session = FakeInferSession::scripted([]);
    let memfd = std::fs::File::open("/dev/null").expect("a descriptor");
    session
        .send_attached(ClientFrame::Cancel, vec![memfd.into()])
        .await
        .expect("send");
    assert_eq!(session.attachment_counts(), [1]);
    assert_eq!(session.sent(), [ClientFrame::Cancel]);
}
