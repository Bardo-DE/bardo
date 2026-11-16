//! Running a plan stage by stage.
//!
//! Each stage is announced before it runs (`Why` when the person
//! asked to see reasons, `Routed` for who it is sent to, and a `Stage` note for the footer), then
//! run by the seam that serves its role: a [`Transcriber`] for `Hear`, a turn runner for `Answer`.
//! The transcript joins the request as text, so the answer stage sees what the person said as
//! words and nothing else about the audio.

use inference_core::{
    AudioFrame, ChatMessage, ChatRequest, ChatSink, InferEvent, InferReply, InferRequest,
    MessagePart, ModelError, ModelLabel, ModelRef, Pipeline, Refusal, Role, Stage, StageNote,
    StageRole, TranscribeBegin, TranscribeReply, Why,
};
use inference_router::audio::AudioIn;
use inference_router::seams::{RunningTurn, TurnRunner, TurnStep};
use std::collections::{BTreeMap, VecDeque};
use std::ops::ControlFlow;

/// What a speech-to-text stage runs on (stoker's `SpeechToText` behind the engine the stage's
/// model names; a scripted one in tests). It is inferd's `SpeechRunner::transcribe`'s shape.
pub trait Transcriber: Send + Sync {
    /// Reads `audio` until it ends, sends `Heard` events to `sink`, and gives the transcript.
    fn transcribe<A: AudioIn, S: ChatSink>(
        &self,
        stage: &Stage,
        begin: &TranscribeBegin,
        audio: &mut A,
        sink: &mut S,
    ) -> impl Future<Output = Result<TranscribeReply, ModelError>> + Send;
}

/// Audio that is already all here.
#[derive(Debug, Clone, Default)]
pub struct VecAudio(pub VecDeque<AudioFrame>);

impl AudioIn for VecAudio {
    fn next(&mut self) -> impl Future<Output = Option<AudioFrame>> + Send {
        std::future::ready(self.0.pop_front())
    }
}

/// What the request carries into the pipeline.
#[derive(Debug, Clone)]
pub struct PipelineInput {
    /// The audio, when the request has any.
    pub audio: Option<(TranscribeBegin, VecAudio)>,
    /// The chat the answering model is given; a transcript is appended to its last user message.
    pub chat: ChatRequest,
    /// The label of each model a stage may name (the catalogue entry's), for the `Stage` notes.
    pub names: BTreeMap<ModelRef, ModelLabel>,
}

/// Why a plan cannot run here yet, as a typed refusal (none for a plan this build runs).
#[must_use]
pub fn unsupported(pipeline: &Pipeline) -> Option<Refusal> {
    pipeline.stages.iter().find_map(|stage| match stage.role {
        StageRole::Describe => Some(Refusal::NotYet("describing images with the image_in slot")),
        StageRole::Speak => Some(Refusal::NotYet(
            "speaking the answer with the voice_out slot",
        )),
        StageRole::Hear | StageRole::Answer => None,
    })
}

fn announce(
    stage: &Stage,
    names: &BTreeMap<ModelRef, ModelLabel>,
    show: bool,
    sink: &mut impl ChatSink,
) -> ControlFlow<()> {
    let evicting = matches!(stage.picked.why, Why::Evicted { .. });
    let why = (show || evicting).then(|| InferEvent::Why(stage.picked.why.clone()));
    let note = InferEvent::Stage(StageNote {
        role: stage.role,
        served: stage.served(),
        why: stage.picked.why.clone(),
        name: names
            .get(&ModelRef {
                account: stage.picked.chosen.account.clone(),
                model: stage.picked.chosen.model.clone(),
            })
            .cloned(),
    });
    let events = why
        .into_iter()
        .chain([InferEvent::Routed(stage.served()), note]);
    for event in events {
        if sink.event(event) == ControlFlow::Break(()) {
            return ControlFlow::Break(());
        }
    }
    ControlFlow::Continue(())
}

/// The chat with `transcript` joined to its last user message (or as a new one).
fn with_transcript(mut chat: ChatRequest, transcript: String) -> ChatRequest {
    let part = MessagePart::Text(transcript);
    match chat
        .messages
        .iter_mut()
        .rev()
        .find(|message| message.role == Role::User)
    {
        Some(message) => message.parts.push(part),
        None => chat.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![part],
        }),
    }
    chat
}

/// Runs `pipeline`: `Hear` through `hear`, then `Answer` through `answer`. A stage this build
/// does not run is refused up front (`unsupported`), before any stage is announced or run.
pub async fn run_pipeline<H: Transcriber, T: TurnRunner>(
    pipeline: &Pipeline,
    show: bool,
    input: PipelineInput,
    hear: &H,
    answer: &T,
    sink: &mut impl ChatSink,
) -> InferReply {
    if unsupported(pipeline).is_some() {
        return InferReply::Refused(inference_core::InferRefusal::Unsupported);
    }
    let PipelineInput {
        mut audio,
        mut chat,
        names,
    } = input;
    for stage in &pipeline.stages {
        if announce(stage, &names, show, sink) == ControlFlow::Break(()) {
            return InferReply::Cancelled;
        }
        match stage.role {
            StageRole::Hear => {
                let Some((begin, frames)) = audio.as_mut() else {
                    return InferReply::Refused(inference_core::InferRefusal::Unsupported);
                };
                match hear.transcribe(stage, begin, frames, sink).await {
                    Ok(heard) => chat = with_transcript(chat, heard.text),
                    Err(error) => return InferReply::Failed(error),
                }
            }
            StageRole::Answer => return answer_turn(answer, chat, sink).await,
            StageRole::Describe | StageRole::Speak => {
                return InferReply::Refused(inference_core::InferRefusal::Unsupported);
            }
        }
    }
    InferReply::Refused(inference_core::InferRefusal::Unavailable)
}

async fn answer_turn<T: TurnRunner>(
    answer: &T,
    chat: ChatRequest,
    sink: &mut impl ChatSink,
) -> InferReply {
    let mut turn = answer.start(InferRequest::Chat(chat), Vec::new());
    loop {
        match turn.next().await {
            TurnStep::Event(event) => {
                if sink.event(event) == ControlFlow::Break(()) {
                    return InferReply::Cancelled;
                }
            }
            TurnStep::Done(reply) => return reply,
        }
    }
}
