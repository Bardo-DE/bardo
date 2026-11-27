//! Builders the step tables share.

pub(super) use crate::session::{
    AudioCursor, ComputerUseProgress, Phase, RoutedNote, Routing, SessionIn, SessionOut,
    SessionSpec, step,
};
use account_capability::ModelId;
use account_capability::{ComputerUseEnv, LanguageTag, SpeechMode};
use account_capability::{Locality, Tier};
use account_core::AccountId;
use account_core::DataClass;
use account_core::consent::Usage;
use account_match::Need;
use account_match::{ComputerUseNeed, LlmNeed, SpeechNeed};
use base_ids::Tokens;
use inference_core::{
    AudioFrame, AudioRate, Base64Bytes, ClientFrame, ComputerUseBegin, ComputerUseStepReply,
    InferEvent, InferRefusedError, InferReply, InferRequest, LangPick, ModelError, ModelRef,
    Readiness, RequestKind, ServedBy, SpeakRequest, Task, TaskRequest, TranscribeBegin,
    TranscribeMode, Why,
};

pub(super) fn model() -> ModelRef {
    ModelRef {
        account: AccountId::parse("local").expect("account"),
        model: ModelId::parse("qwen").expect("model"),
    }
}

pub(super) fn served() -> ServedBy {
    ServedBy {
        account: AccountId::parse("local").expect("account"),
        model: ModelId::parse("qwen").expect("model"),
        locality: Locality::OnDevice,
    }
}

pub(super) fn label() -> inference_core::ModelLabel {
    inference_core::ModelLabel("Qwen".into())
}

/// The `Answer` note of a turn on `served()` as routed by `decided`.
pub(super) fn answer() -> inference_core::StageNote {
    answer_for(Why::Named)
}

/// The `Answer` note of a turn routed for this reason.
pub(super) fn answer_for(why: Why) -> inference_core::StageNote {
    inference_core::StageNote {
        role: inference_core::StageRole::Answer,
        served: served(),
        why,
        name: Some(label()),
    }
}

pub(super) fn answer_event() -> SessionOut {
    SessionOut::Emit(InferEvent::Stage(answer()))
}

pub(super) fn decided(readiness: Readiness) -> SessionIn {
    SessionIn::Routed(Ok(Routing {
        served: served(),
        readiness,
        why: Why::Named,
        reached: None,
        show: false,
        name: Some(label()),
    }))
}

pub(super) fn decided_why(readiness: Readiness, why: Why, show: bool) -> SessionIn {
    SessionIn::Routed(Ok(Routing {
        served: served(),
        readiness,
        why,
        reached: None,
        show,
        name: Some(label()),
    }))
}

pub(super) fn routed() -> SessionOut {
    SessionOut::Emit(InferEvent::Routed(served()))
}

pub(super) fn llm_spec(class: DataClass) -> SessionSpec {
    SessionSpec {
        need: Need::Llm(LlmNeed::new(std::collections::BTreeSet::new(), Tokens(1))),
        class,
        tier: Tier::Balanced,
        usage: account_core::consent::Usage::Interactive,
    }
}

pub(super) fn speech_spec(class: DataClass) -> SessionSpec {
    SessionSpec {
        need: Need::Speech(SpeechNeed::new([SpeechMode::Stt, SpeechMode::Tts].into())),
        class,
        tier: Tier::Fast,
        usage: account_core::consent::Usage::Interactive,
    }
}

pub(super) fn computer_use_spec(class: DataClass) -> SessionSpec {
    SessionSpec {
        need: Need::ComputerUse(ComputerUseNeed::new([ComputerUseEnv::Desktop].into())),
        class,
        tier: Tier::Best,
        usage: account_core::consent::Usage::Interactive,
    }
}

pub(super) fn task(class: DataClass) -> InferRequest {
    InferRequest::Task(TaskRequest::new(
        Task::Summarise,
        "text".into(),
        class,
        Usage::Interactive,
    ))
}

pub(super) fn transcribe() -> InferRequest {
    InferRequest::Transcribe(TranscribeBegin::new(
        TranscribeMode::Streaming,
        LangPick::Auto,
        AudioRate(16_000),
        Usage::Interactive,
    ))
}

pub(super) fn speak(class: DataClass) -> InferRequest {
    InferRequest::Speak(SpeakRequest::new(
        "hi".into(),
        LanguageTag::parse("en").expect("tag"),
        class,
        Usage::Interactive,
    ))
}

pub(super) fn computer_use_begin() -> InferRequest {
    InferRequest::ComputerUseBegin(ComputerUseBegin::new(
        "rename".into(),
        ComputerUseEnv::Desktop,
    ))
}

pub(super) fn audio(at: u64, samples: usize) -> ClientFrame {
    ClientFrame::Audio(AudioFrame {
        at,
        pcm: Base64Bytes(vec![0; samples * 2]),
    })
}

pub(super) fn waiting(queued: Option<InferRequest>) -> Phase {
    waiting_for(answer(), queued)
}

/// Waiting on a model routed with this note.
pub(super) fn waiting_for(
    answer: inference_core::StageNote,
    queued: Option<InferRequest>,
) -> Phase {
    Phase::Waiting {
        served: served(),
        answer,
        queued,
    }
}

pub(super) fn idle(routed: RoutedNote) -> Phase {
    idle_computer_use(routed, ComputerUseProgress::NotBegun)
}

pub(super) fn idle_computer_use(routed: RoutedNote, computer_use: ComputerUseProgress) -> Phase {
    Phase::Idle {
        served: served(),
        answer: answer(),
        routed,
        computer_use,
    }
}

pub(super) fn in_turn(kind: RequestKind, audio: AudioCursor) -> Phase {
    in_turn_with(kind, audio, ComputerUseProgress::NotBegun, None)
}

pub(super) fn in_turn_with(
    kind: RequestKind,
    audio: AudioCursor,
    computer_use: ComputerUseProgress,
    queued: Option<InferRequest>,
) -> Phase {
    Phase::InTurn {
        served: served(),
        answer: answer(),
        kind,
        audio,
        computer_use,
        queued,
    }
}

pub(super) fn queued_chat(queued: Option<InferRequest>) -> Phase {
    in_turn_with(
        RequestKind::Chat,
        AudioCursor::NoAudio,
        ComputerUseProgress::NotBegun,
        queued,
    )
}

pub(super) fn lost_reply() -> InferReply {
    InferReply::Failed(ModelError::Unreachable)
}

pub(super) fn begun_ack() -> InferReply {
    InferReply::ComputerUseStep(ComputerUseStepReply::new(vec![]))
}

pub(super) fn computer_use_step() -> InferRequest {
    InferRequest::ComputerUseStep(inference_core::ComputerUseStepRequest::new(
        inference_core::StepIndex(0),
        inference_core::WindowGeometry {
            logical: computer_use_geometry::Size::new(
                computer_use_geometry::Coord(1),
                computer_use_geometry::Coord(1),
            ),
            scale: computer_use_geometry::Scale120(120),
        },
        inference_core::FrameImage {
            source: inference_core::ImageSource::Attached(inference_core::AttachIndex(0)),
            layout: inference_core::FrameLayout::Encoded(inference_core::MediaType::Png),
        },
        None,
    ))
}

pub(super) fn expecting(next: u64) -> AudioCursor {
    AudioCursor::Expecting { next }
}

pub(super) fn done(reply: InferReply) -> SessionOut {
    SessionOut::Emit(InferEvent::Finished(reply))
}

pub(super) fn unsupported() -> SessionOut {
    done(InferReply::Refused(InferRefusedError::Unsupported))
}

pub(super) fn frame(f: ClientFrame) -> SessionIn {
    SessionIn::Frame(f)
}

pub(super) fn request(r: InferRequest) -> SessionIn {
    frame(ClientFrame::Request(r))
}

pub(super) type Row = (
    &'static str,
    SessionSpec,
    Phase,
    SessionIn,
    Phase,
    Vec<SessionOut>,
);
