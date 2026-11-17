//! One computer-use step over a model turn.
//!
//! The model layer's `ComputerUseSession` builds the prompt and reads the
//! reply, and this module is the turn between them. It prepares the frame, runs the turn through
//! a transcript sink, hands the transcript to `absorb_for` and, when that asks for a repair, runs
//! one more turn with the request it returns.
//!
//! What the model layer leaves to the daemon is done here, from the model's catalog entry: the
//! `ComputerUseProfile` (dialect, resize rule and space of the entry, a history of one frame fewer than
//! the model takes per prompt, one repair) and the `TurnOptions` (the entry's output limit, how
//! many calls a turn may make, the engine's extras, and reasoning off with its sampling, except for
//! a model trained to think before its action, which runs with the engine's reasoning and the
//! sampling for it).
//!
//! The window's contents and the notes said to the run are `ObservationIn::with_tree` and
//! `with_notes`; a note keeps who said it in its words ("The person says: ..."), because the model layer's
//! `StepNote` is one line of text.

use crate::bridge::{self, BridgeError, Frames};
use crate::tee::{Echo, Tee};
use base_ids::Tokens;
use computer_use_action::{AnswerShape, ComputerUseDialect};
use computer_use_session::{
    ComputerUseProfile, ComputerUseSession, ComputerUseTaskText, FrameBudget, ObservationIn,
    RepairBudget, StepLines, StepNote as NoteLine, StepOutcome, TurnOptions,
};
use computer_use_vendors::StepResult;
use inference_core::{
    ComputerUseBegin, ComputerUseStepFailure, ComputerUseStepReply, ComputerUseStepRequest,
    DroppedAction, FrameLayout, InferEvent, ModelError, NoteFrom, PrevResult, StepNote,
};
use inference_router::local::LocalModel;
use model_provider::{
    Batching, ComputerUseSupport, ImageInput, Limits, Provider, Reasoning, ToolCallId,
    ToolParallelism,
};
use model_vision_prep::{Encoding, FrameMap, RawFrame, prepare};
use std::ops::ControlFlow;

/// A step that gave no actions: why the app is told. A reply that never parsed is a step the model
/// took, which the session (changed in place) already lists; any other failure leaves it as it was.
#[derive(Debug)]
pub struct Failed {
    /// What the app is told.
    pub failure: ComputerUseStepFailure,
}

impl From<ComputerUseStepFailure> for Failed {
    fn from(failure: ComputerUseStepFailure) -> Self {
        Self { failure }
    }
}

fn unreadable() -> Failed {
    ComputerUseStepFailure::ModelFailed(ModelError::Unreadable).into()
}

/// The session for a run on this model, or none when the model has no tool or text dialect.
///
/// A vendor's wire needs the vendor's backend, not an engine on this computer.
/// A reply is never asked to be longer than `ceiling`.
#[must_use]
pub fn open(
    model: &LocalModel,
    begin: &ComputerUseBegin,
    ceiling: Tokens,
) -> Option<ComputerUseSession> {
    let caps = model.caps()?;
    let ComputerUseSupport::Dialect {
        dialect,
        batching,
        profile: actions,
        ..
    } = caps.computer_use
    else {
        return None;
    };
    if matches!(dialect, ComputerUseDialect::Wire(_)) {
        return None;
    }
    // As many past frames as the model takes with the current one (`per_prompt - 1`); the model layer cuts
    // the wish to what the entry allows.
    let profile = ComputerUseProfile::for_model(
        dialect,
        &caps.images,
        FrameBudget::within(caps.images.per_prompt),
        RepairBudget(1),
        Encoding::Png,
    )
    // The model is shown, and read through, its own action names and drag and scroll styles.
    .with_actions(actions);
    // A model trained to think before it acts is run with its reasoning; any other answers plainly.
    let reasoning = match actions.answer {
        AnswerShape::ThinkActionCall => Reasoning::EngineDefault,
        AnswerShape::Free => Reasoning::Off,
    };
    let settings = TurnOptions {
        sampling: *model.sampling()?.for_reasoning(reasoning),
        limits: Limits {
            max_output: caps.max_output.min(ceiling),
            stop: Vec::new(),
            // The session that runs the steps bounds them; a step has no deadline of its own.
            deadline: None,
        },
        reasoning,
        tool_calls: match batching {
            Batching::One => ToolParallelism::One,
            Batching::Many => ToolParallelism::Many,
        },
        engine: bridge::extras(model.flavor),
        lines: StepLines(8),
    };
    let task = ComputerUseTaskText {
        goal: begin.goal.clone(),
        hints: begin.hints.clone(),
    };
    Some(ComputerUseSession::begin(profile, task, model.name.clone()).with_settings(settings))
}

/// What happened to each action of the previous step, as the model layer's prompt has it. The calls'
/// ids are ours (a tool or text dialect does not read them; a vendor wire is not served here).
/// A failure, and the person's own part in it, are told as a refusal with their words.
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

/// What the runner saw before this step, with the window's contents and the notes when there are
/// any.
fn observation(request: &ComputerUseStepRequest) -> ObservationIn {
    let seen = ObservationIn::new(
        request.step,
        request.cursor,
        prev_results(&request.prev),
        request.masked,
    )
    .with_notes(request.notes.iter().map(note_line).collect());
    match &request.tree {
        Some(tree) => seen.with_tree(tree.clone()),
        None => seen,
    }
}

/// The frame as an image the model is shown.
fn frame_image(
    request: &ComputerUseStepRequest,
    frames: &Frames,
    map: &FrameMap,
) -> Result<ImageInput, BridgeError> {
    let bytes = frames.resolve(&request.frame.source)?;
    match request.frame.layout {
        FrameLayout::Encoded(media) => Ok(bridge::image_input(media, bytes)),
        FrameLayout::Raw {
            format,
            size,
            stride,
        } => {
            let raw = RawFrame {
                pixels: &bytes,
                size,
                stride,
                format,
            };
            #[expect(
                clippy::map_err_ignore,
                reason = "an image that cannot be prepared is one Attachment error; the app is told no more"
            )]
            let prepared = prepare(raw, map, Encoding::Png).map_err(|_| BridgeError::Attachment)?;
            Ok(bridge::image_input(prepared.media, prepared.bytes))
        }
    }
}

/// One step on the run's own session, which the step changes in place.
///
/// The reply, with the step
/// remembered. Thoughts stream into `forward` as the turn runs, then an `ActionProposed` for each
/// action; a `Stop` from it ends the turn (nobody is listening) or the proposals. A step that
/// fails before its reply is absorbed leaves the session's history as it was, and one that fails
/// after a repair gets its repair budget back, so the same step can be asked again.
///
/// # Errors
///
/// Returns [`Failed`] when the model has no readable catalog entry, the frame cannot be prepared,
/// the turn fails or is cut, or the reply never parses.
pub async fn step<P: Provider>(
    session: &mut ComputerUseSession,
    model: &LocalModel,
    provider: &P,
    request: &ComputerUseStepRequest,
    frames: &Frames,
    mut forward: impl FnMut(InferEvent) -> ControlFlow<()> + Send,
) -> Result<ComputerUseStepReply, Failed> {
    let caps = model.caps().ok_or_else(unreadable)?;
    #[expect(
        clippy::map_err_ignore,
        reason = "a frame map that cannot be built is one Unreadable failure; the app is told no more"
    )]
    let map = FrameMap::new(
        request.window.logical,
        request.window.scale,
        &caps.images.rule,
        caps.images.space,
    )
    .map_err(|_| unreadable())?;
    #[expect(
        clippy::map_err_ignore,
        reason = "a frame that cannot be prepared is one Unreadable failure; the app is told no more"
    )]
    let image = frame_image(request, frames, &map).map_err(|_| unreadable())?;
    let mut sent = session.request(&observation(request), &map, image);
    loop {
        let mut tee = Tee::new(Echo::ThoughtsAndText, &mut forward);
        let end = match provider.turn(&sent, &mut tee).await {
            Ok(end) => end,
            Err(error) => {
                session.refill_repairs();
                return Err(
                    ComputerUseStepFailure::ModelFailed(bridge::model_error(&error)).into(),
                );
            }
        };
        if tee.flow() == ControlFlow::Break(()) {
            session.refill_repairs();
            // The session is gone: the cut turn holds no call, and a repair would be asked of nobody.
            return Err(ComputerUseStepFailure::Unparseable.into());
        }
        // Where the cursor was: a model that drags from the cursor needs it, or its drags drop.
        let outcome = session.absorb_at(&sent, tee.finish(end), &map, request.cursor);
        match outcome {
            StepOutcome::Actions {
                thought,
                actions,
                dropped,
            } => {
                for action in &actions {
                    if forward(InferEvent::ActionProposed(action.clone())) == ControlFlow::Break(())
                    {
                        break;
                    }
                }
                let mut reply = ComputerUseStepReply::new(actions).with_dropped(
                    dropped
                        .iter()
                        .map(|d| DroppedAction {
                            verb: d.verb.as_str().to_owned(),
                            reason: d.reason,
                        })
                        .collect(),
                );
                if let Some(thought) = thought {
                    reply = reply.with_thought(thought);
                }
                return Ok(reply);
            }
            StepOutcome::Repair(next) => sent = next,
            StepOutcome::Unparseable(_) => {
                return Err(Failed {
                    failure: ComputerUseStepFailure::Unparseable,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests;
