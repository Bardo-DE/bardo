use super::*;
use crate::testkit::models;
use account_capability::Locality;
use account_capability::ModelId;
use account_core::AccountId;
use account_core::DataClass;
use account_core::consent::Usage;
use inference_core::{
    ChatMessage, ChatRequest, ImagePart, ImageSource, MessagePart, Role, SpeakRequest,
    TranscribeBegin, TranscribeMode, VoiceName,
};
use inference_router::seams::RunningTurn;
use base_testkit::Scratch;
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;

fn served() -> ServedBy {
    ServedBy {
        account: AccountId::parse("local").expect("id"),
        model: ModelId::parse("tiny-chat").expect("id"),
        locality: Locality::OnDevice,
    }
}

fn chat_with(parts: Vec<MessagePart>) -> InferRequest {
    InferRequest::Chat(ChatRequest::new(
        vec![ChatMessage {
            role: Role::User,
            parts,
        }],
        Tier::Fast,
        DataClass::Notes,
        Usage::Interactive,
    ))
}

fn chat() -> InferRequest {
    chat_with(vec![MessagePart::Text("hi".into())])
}

fn turns(scratch: &Scratch, which: usize) -> (Turns, Arc<LocalModel>) {
    let model = Arc::new(models(scratch).remove(which));
    let pin = Pin::new();
    pin.set(Pinned {
        served: served(),
        model: Some(Arc::clone(&model)),
        cloud: None,
    });
    (Turns::new(pin, Supervised::idle(), Tier::Fast), model)
}

async fn finish(turns: &Turns, request: InferRequest) -> InferReply {
    let mut turn = turns.start(request, Vec::new());
    loop {
        if let TurnStep::Done(reply) = turn.next().await {
            return reply;
        }
    }
}

#[tokio::test]
async fn a_turn_before_any_route_has_nothing_to_run_on() {
    let turns = Turns::new(Pin::new(), Supervised::idle(), Tier::Fast);
    assert_eq!(
        finish(&turns, chat()).await,
        InferReply::Failed(ModelError::Unreachable)
    );
}

#[tokio::test]
async fn a_pin_keeps_its_first_decision() {
    let pin = Pin::new();
    pin.set(Pinned {
        served: served(),
        model: None,
        cloud: None,
    });
    let mut other = served();
    other.model = ModelId::parse("other").expect("id");
    pin.set(Pinned {
        served: other,
        model: None,
        cloud: None,
    });
    assert_eq!(
        pin.get().map(|p| p.served.model.as_str().to_owned()),
        Some("tiny-chat".into())
    );
}

#[tokio::test(start_paused = true)]
async fn an_engine_that_is_not_listening_is_unreachable_after_the_retries() {
    let scratch = Scratch::new("run-down");
    let (turns, _) = turns(&scratch, 0);
    assert_eq!(
        finish(&turns, chat()).await,
        InferReply::Failed(ModelError::Unreachable)
    );
}

#[tokio::test]
async fn kinds_the_runner_cannot_serve_are_refused_unsupported_and_so_is_a_step_before_a_begin() {
    let scratch = Scratch::new("run-unsupported");
    let (turns, _) = turns(&scratch, 2);
    let cases = [
        InferRequest::Transcribe(TranscribeBegin::new(
            TranscribeMode::Batch,
            inference_core::LangPick::Auto,
            inference_core::AudioRate(16_000),
            Usage::Interactive,
        )),
        InferRequest::Speak(
            SpeakRequest::new(
                "hi".into(),
                account_capability::LanguageTag::parse("en").expect("tag"),
                DataClass::Voice,
                Usage::Interactive,
            )
            .with_voice(VoiceName("af_heart".into())),
        ),
    ];
    for request in cases {
        assert_eq!(
            finish(&turns, request).await,
            InferReply::Refused(InferRefusal::Unsupported)
        );
    }
    // An image type with no encoder never reaches the engine.
    let gif = chat_with(vec![MessagePart::Image(ImagePart {
        media_type: "image/gif".into(),
        source: ImageSource::Inline(inference_core::Base64Bytes(vec![1])),
    })]);
    assert_eq!(
        finish(&turns, gif).await,
        InferReply::Refused(InferRefusal::Unsupported)
    );
}

#[tokio::test]
async fn a_model_with_no_embedding_details_refuses_to_embed() {
    let scratch = Scratch::new("run-embed");
    let (turns, _) = turns(&scratch, 0);
    let request = InferRequest::Embed(inference_core::EmbedRequest::new(
        vec!["a".into()],
        inference_core::EmbedRole::Query,
        account_match::DimsNeed::Any,
        DataClass::Notes,
        Usage::Background,
    ));
    assert_eq!(
        finish(&turns, request).await,
        InferReply::Refused(InferRefusal::Unsupported)
    );
}

#[tokio::test]
async fn dropping_a_running_turn_closes_the_engines_connection() {
    let scratch = Scratch::new("run-drop");
    let (turns, model) = turns(&scratch, 0);
    std::fs::create_dir_all(model.socket.0.parent().expect("dir")).expect("dir");
    let listener = UnixListener::bind(&model.socket.0).expect("bind");
    let turn = turns.start(chat(), Vec::new());
    // The engine accepts, reads the request, and never answers.
    let (mut engine, _) = listener.accept().await.expect("accept");
    let mut buf = [0_u8; 4096];
    assert!(engine.read(&mut buf).await.expect("read") > 0);
    drop(turn);
    // Dropping the turn aborts its task and closes the socket: the engine sees the end.
    let end = tokio::time::timeout(base_testkit::GENEROUS, async {
        loop {
            match engine.read(&mut buf).await {
                Ok(0) | Err(_) => return,
                Ok(_) => {}
            }
        }
    })
    .await;
    assert!(end.is_ok(), "the engine's stream was closed");
}
