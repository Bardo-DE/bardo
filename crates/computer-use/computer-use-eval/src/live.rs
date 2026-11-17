//! A real computer-use model, reached the way inferd reaches one.
//!
//! A real computer-use model, reached the way inferd reaches one: the model layer's `ComputerUseSession` builds
//! each step's prompt from the model's catalogue entry, a provider (an engine on this machine,
//! over loopback HTTP) runs the turn, and the session reads the reply into actions. This is the
//! same turn inferd makes, without inferd, so the eval needs no daemon and no bus.
//!
//! The entry is read from a file the caller names (the model layer's catalogue); nothing here chooses a
//! model, an engine or an address.

use base_ids::Tokens;
use computer_use_action::{ComputerUseDialect, Size, WindowSpace};
use computer_use_run::{
    ComputerUseModel, ComputerUseModelError, FrameAttachment, StepEvent, StepSink, ThoughtText,
};
use computer_use_session::{
    ComputerUseProfile, ComputerUseSession, ComputerUseTaskText, FrameBudget, MaskedRegions,
    ObservationIn, RepairBudget, StepIndex, StepLines, StepNote as NoteLine, StepOutcome,
    TranscriptSink, TurnSettings,
};
use computer_use_vendors::StepResult;
use inference_core::{
    ComputerUseBegin, ComputerUseStepReply, ComputerUseStepRequest, DropReason, DroppedAction,
    NoteFrom, PrevResult, ServedBy, StepNote, TurnUsage,
};
use model_catalog::{ModelEntry, parse_entry};
use model_provider::{
    Batching, ComputerUseSupport, EngineExtras, ImageBytes, ImageDetail, ImageInput, Limits,
    ModelName, Provider, ProviderError, Reasoning, Sampling, ToolCallId, ToolParallelism,
};
use model_vision_prep::{Encoding, FrameMap, RawFrame, prepare};
use std::sync::Arc;

/// Why a model could not be set up.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LiveError {
    /// The entry file is unreadable or not a catalogue entry.
    #[error("catalogue entry: {0}")]
    Entry(String),
    /// The entry is not a computer-use model with a tool or text dialect.
    #[error("the entry is not a computer-use model this eval can drive")]
    NotComputerUse,
}

/// Whether the model thinks before it acts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Thinking {
    /// No: the entry's greedy sampling, the same screen gives the same click.
    #[default]
    Off,
    /// Yes: the entry's thinking sampling.
    On,
}

/// What the entry says about driving the model.
#[derive(Debug, Clone)]
pub struct Plan {
    profile: ComputerUseProfile,
    settings: TurnSettings,
    name: ModelName,
}

impl Plan {
    /// The plan for the entry in `text`, served by the engine under `served_name`.
    ///
    /// # Errors
    ///
    /// Returns a [`LiveError`] when `text` is not a catalogue entry or has no tool-using computer-use
    /// profile.
    pub fn of(text: &str, served_name: &str, thinking: Thinking) -> Result<Plan, LiveError> {
        let entry: ModelEntry = parse_entry(text).map_err(|e| LiveError::Entry(e.to_string()))?;
        Plan::from_entry(&entry, served_name, thinking)
    }

    fn from_entry(
        entry: &ModelEntry,
        served_name: &str,
        thinking: Thinking,
    ) -> Result<Plan, LiveError> {
        let caps = &entry.capabilities;
        let images = caps.image_in.ok_or(LiveError::NotComputerUse)?;
        let text_out = caps.text_out.as_ref().ok_or(LiveError::NotComputerUse)?;
        let sampling = text_out.sampling.ok_or(LiveError::NotComputerUse)?;
        let ComputerUseSupport::Dialect {
            dialect,
            batching,
            profile: actions,
            ..
        } = caps.actions_out.ok_or(LiveError::NotComputerUse)?
        else {
            return Err(LiveError::NotComputerUse);
        };
        if matches!(dialect, ComputerUseDialect::Wire(_)) {
            return Err(LiveError::NotComputerUse);
        }
        let profile = ComputerUseProfile::for_model(
            dialect,
            &images,
            FrameBudget::within(images.per_prompt),
            RepairBudget(1),
            Encoding::Png,
        )
        .with_actions(actions);
        let (sampling, reasoning): (Sampling, Reasoning) = match thinking {
            Thinking::Off => (sampling.reasoning_off, Reasoning::Off),
            Thinking::On => (sampling.reasoning_on, Reasoning::EngineDefault),
        };
        let settings = TurnSettings {
            sampling,
            limits: Limits {
                max_output: text_out.max_output,
                stop: Vec::new(),
                deadline: None,
            },
            reasoning,
            tool_calls: match batching {
                Batching::One => ToolParallelism::One,
                Batching::Many => ToolParallelism::Many,
            },
            engine: EngineExtras::None,
            lines: StepLines(8),
        };
        Ok(Plan {
            profile,
            settings,
            name: ModelName(served_name.to_owned()),
        })
    }
}

/// A [`ComputerUseModel`] over a provider.
#[derive(Debug)]
pub struct ProviderModel<P> {
    provider: Arc<P>,
    plan: Plan,
    session: Option<ComputerUseSession>,
    served: ServedBy,
    usage: TurnUsage,
}

impl<P: Provider> ProviderModel<P> {
    /// A model that runs `plan` on `provider`, saying it is `served` by this machine.
    pub fn new(provider: Arc<P>, plan: Plan, served: ServedBy) -> ProviderModel<P> {
        ProviderModel {
            provider,
            plan,
            session: None,
            served,
            usage: TurnUsage {
                input: Tokens(0),
                output: Tokens(0),
                cached: Tokens(0),
                charge: None,
                ..TurnUsage::default()
            },
        }
    }
}

/// What a turn said, whole: its thinking, its words and its calls.
fn said_in(turn: &computer_use_session::TurnTranscript) -> String {
    let calls: Vec<&str> = turn.calls.iter().map(|c| c.input.as_str()).collect();
    format!("{} {} {}", turn.thought, turn.text, calls.join(" "))
}

fn model_error(error: &ProviderError) -> ComputerUseModelError {
    match error {
        ProviderError::Unreachable
        | ProviderError::NotReady
        | ProviderError::Timeout
        | ProviderError::RateLimited(_)
        | ProviderError::Server(_) => ComputerUseModelError::Transient,
        ProviderError::Unreadable(_) => ComputerUseModelError::Parse,
        _ => ComputerUseModelError::Fatal,
    }
}

fn prev_results(prev: &[PrevResult]) -> Vec<StepResult> {
    prev.iter()
        .enumerate()
        .map(|(index, result)| {
            let id = ToolCallId(format!("prev-{index}"));
            match result {
                PrevResult::Done => StepResult::Done(id),
                PrevResult::NotRun => StepResult::NotRun(id),
                PrevResult::Refused(why) | PrevResult::Failed(why) => StepResult::Refused {
                    id,
                    why: why.clone(),
                },
                PrevResult::UserDeclined => StepResult::Refused {
                    id,
                    why: "the person declined it".to_owned(),
                },
                PrevResult::UserActed => StepResult::Refused {
                    id,
                    why: "the person did it themselves".to_owned(),
                },
            }
        })
        .collect()
}

fn note_line(note: &StepNote) -> NoteLine {
    let who = match note.from {
        NoteFrom::Person => "The person says",
        NoteFrom::Agent => "A helper says",
    };
    NoteLine(format!("{who}: {}", note.text))
}

fn observation(request: &ComputerUseStepRequest) -> ObservationIn {
    let seen = ObservationIn::new(
        StepIndex(request.step.0),
        request.cursor,
        prev_results(&request.prev),
        MaskedRegions(request.masked.0),
    )
    .with_notes(request.notes.iter().map(note_line).collect());
    match &request.tree {
        Some(tree) => seen.with_tree(tree.clone()),
        None => seen,
    }
}

fn dropped_reason(reason: computer_use_parse::DropReason) -> DropReason {
    use computer_use_parse::DropReason as P;
    match reason {
        P::UnsupportedVerb => DropReason::UnsupportedVerb,
        P::MissingArgument => DropReason::MissingArgument,
        P::BadArgument => DropReason::BadArgument,
        P::BadNumber => DropReason::BadNumber,
        P::TooLong => DropReason::TooLong,
        P::OverBatchLimit => DropReason::OverBatchLimit,
        P::OutOfFrame => DropReason::OutOfFrame,
    }
}

impl<P: Provider + 'static> ComputerUseModel for ProviderModel<P> {
    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn begin(&mut self, begin: ComputerUseBegin) -> Result<(), ComputerUseModelError> {
        let task = ComputerUseTaskText {
            goal: begin.goal,
            hints: begin.hints,
        };
        let session =
            ComputerUseSession::begin(self.plan.profile.clone(), task, self.plan.name.clone())
                .with_settings(self.plan.settings.clone());
        self.session = Some(session);
        Ok(())
    }

    async fn step(
        &mut self,
        request: ComputerUseStepRequest,
        frame: FrameAttachment,
        sink: &mut impl StepSink,
    ) -> Result<ComputerUseStepReply, ComputerUseModelError> {
        let session = self.session.as_mut().ok_or(ComputerUseModelError::Fatal)?;
        let window: Size<WindowSpace> = request.window.logical;
        let map = FrameMap::new(
            window,
            request.window.scale,
            &self.plan.profile.rule,
            self.plan.profile.space,
        )
        .map_err(|error| {
            tracing::warn!(%error, "could not map the window to the model's frame");
            ComputerUseModelError::Fatal
        })?;
        let pixels = &frame.0;
        let raw = RawFrame {
            pixels: &pixels.data,
            size: pixels.size,
            stride: pixels.stride,
            format: pixels.format,
        };
        let prepared = prepare(raw, &map, Encoding::Png).map_err(|error| {
            tracing::warn!(%error, "could not prepare the frame for the model");
            ComputerUseModelError::Fatal
        })?;
        let image = ImageInput {
            media: prepared.media,
            bytes: ImageBytes(prepared.bytes),
            detail: ImageDetail::Auto,
        };
        let mut sent = session.request(&observation(&request), &map, image);
        loop {
            let mut collect = TranscriptSink::new();
            let end = match self.provider.turn(&sent, &mut collect).await {
                Ok(end) => end,
                Err(error) => {
                    session.refill_repairs();
                    return Err(model_error(&error));
                }
            };
            self.usage = TurnUsage {
                input: end.usage.input,
                output: end.usage.output,
                charge: end.usage.charge,
                ..TurnUsage::default()
            };
            let turn = collect.finish(end);
            sink.event(StepEvent::ThoughtDelta(ThoughtText(said_in(&turn))));
            match session.absorb_at(&sent, turn, &map, request.cursor) {
                StepOutcome::Actions {
                    thought,
                    actions,
                    dropped,
                } => {
                    let mut reply = ComputerUseStepReply::new(actions).with_dropped(
                        dropped
                            .iter()
                            .map(|d| DroppedAction {
                                verb: d.verb.as_str().to_owned(),
                                reason: dropped_reason(d.reason),
                            })
                            .collect(),
                    );
                    reply.thought = thought;
                    return Ok(reply);
                }
                StepOutcome::Repair(next) => sent = next,
                StepOutcome::Unparseable(_) => return Err(ComputerUseModelError::Parse),
            }
        }
    }

    fn usage(&self) -> TurnUsage {
        self.usage
    }

    fn served(&self) -> Option<ServedBy> {
        Some(self.served.clone())
    }
}
