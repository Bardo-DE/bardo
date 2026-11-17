//! From an observation to the model's turn.

use crate::attempts::Verb;
use crate::observation::{CursorObs, Observation, TreeObs};
use crate::shortcut_guard::control_for;
use action_core::{InboundLine, InboundPart, Reveal};
use computer_use_a11y_tree::{A11yNode, render};
use computer_use_action::{ActionClass, ComputerUseAction, WindowSpace};
use computer_use_bus::{ActionOutcome, ActionResult, LocalRefusal, Refusal};
use computer_use_capture::{PixelBuf, WholeMask};
use inference_core::{
    AttachIndex, ComputerUseStepRequest, FrameImage, FrameLayout, ImageSource, MaskedRegions,
    NoteFrom, PrevResult, StepNote, TreeText, WindowGeometry,
};
use provenance::{AgentRef, Integrity};
use serde::{Deserialize, Serialize};

/// The frame's bytes, sent beside the request as a memfd attached to the inferd session (never
/// base64 on the bus). The request's `FrameImage` points at it by index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FrameAttachment(pub PixelBuf);

/// How an earlier action ended, in the words the model is told: a refusal says only that it was
/// refused (the router's reasons are coarse on purpose and are not an oracle to probe), and a
/// failure names what went wrong with the input.
fn prev_result(outcome: &ActionOutcome, nodes: &[A11yNode], unchanged: bool) -> PrevResult {
    match &outcome.result {
        ActionResult::Done if sent_input(&outcome.action.action) && unchanged => {
            PrevResult::Failed(format!(
                "the {} ran but nothing changed on screen: try something visible instead",
                verb(&outcome.action.action)
            ))
        }
        ActionResult::Done | ActionResult::Answered(_) => PrevResult::Done,
        ActionResult::NotExecuted => PrevResult::NotRun,
        ActionResult::Refused(Refusal::Gate(_)) => PrevResult::Refused("not allowed".to_owned()),
        ActionResult::Refused(Refusal::Local(LocalRefusal::OutsideLease)) => {
            PrevResult::Refused("outside the window".to_owned())
        }
        ActionResult::Refused(Refusal::Local(LocalRefusal::SecretField)) => {
            PrevResult::Refused("a secret field".to_owned())
        }
        ActionResult::Refused(Refusal::Local(LocalRefusal::NotFocused)) => {
            PrevResult::Refused("the window is not focused: click in it first".to_owned())
        }
        ActionResult::Refused(Refusal::Local(LocalRefusal::StateMoved)) => {
            PrevResult::Refused("the window changed".to_owned())
        }
        ActionResult::Refused(Refusal::Local(LocalRefusal::NoProgress)) => {
            PrevResult::Refused(format!(
                "the last {} changed nothing on screen: try something visible instead",
                verb(&outcome.action.action)
            ))
        }
        ActionResult::Refused(Refusal::Local(LocalRefusal::HasControl)) => PrevResult::Refused(
            control_for(&outcome.action.action, nodes)
                .map_or_else(|| "a control does this".to_owned(), |c| c.words()),
        ),
        ActionResult::UserDeclined => PrevResult::UserDeclined,
        ActionResult::Failed(error) => PrevResult::Failed(error.to_string()),
        ActionResult::UserActed { .. } => PrevResult::UserActed,
        ActionResult::CheckFailed => {
            PrevResult::Failed("the success check did not pass".to_owned())
        }
    }
}

/// Whether the action sent input to the window.
pub(crate) fn sent_input(action: &ComputerUseAction<WindowSpace>) -> bool {
    matches!(action.class(), ActionClass::Pointer | ActionClass::Keyboard)
}

/// What the action is called in the words to the model.
fn verb(action: &ComputerUseAction<WindowSpace>) -> &'static str {
    Verb::of(action).map_or("action", Verb::words)
}

/// The note an answer to the model's own question becomes: the person's words, or an agent's
/// when the label does not say they are the person's.
fn answer_note(outcome: &ActionOutcome) -> Option<StepNote> {
    match &outcome.result {
        ActionResult::Answered(answer) => Some(StepNote {
            from: match answer.label.integrity {
                Integrity::Trusted => NoteFrom::Person,
                Integrity::Untrusted => NoteFrom::Agent,
            },
            text: answer.value.clone(),
        }),
        _ => None,
    }
}

/// The note a message that reached the run becomes.
///
/// It holds its plain text parts, one per line, from
/// the person when the sender is the person and from an agent otherwise. A part the router held
/// back as a handle is not text the run may read, and a message with no plain text says
/// nothing. Pure.
#[must_use]
pub fn note_of(line: &InboundLine) -> Option<StepNote> {
    let text: Vec<&str> = line
        .parts
        .iter()
        .filter_map(|part| match part {
            InboundPart::Text(Reveal::Plain(text)) => Some(text.as_str()),
            _ => None,
        })
        .collect();
    if text.is_empty() {
        return None;
    }
    Some(StepNote {
        from: match line.from.agent {
            AgentRef::User => NoteFrom::Person,
            _ => NoteFrom::Agent,
        },
        text: text.join("\n"),
    })
}

/// How many regions were blacked out: each region applied, and the whole frame as one more when
/// it was blacked.
fn masked_regions(obs: &Observation) -> MaskedRegions {
    let whole = match obs.frame.masks.whole {
        WholeMask::None => 0,
        WholeMask::Blacked(_) => 1,
    };
    let count = obs.frame.masks.regions.len().saturating_add(whole);
    MaskedRegions(u16::try_from(count).unwrap_or(u16::MAX))
}

/// The step request for `obs`.
///
/// It holds the window geometry, the frame described and attached, the cursor,
/// how the previous actions ended, how many regions were masked, and the tree rendered to text
/// (secret values never rendered). `notes` is the messages that reached the run since the last
/// step as `StepNote`s, oldest first (empty when none: an agent's note is untrusted text, a
/// person's is their own words, and neither is authority), then the person's answers to the
/// model's own questions. Pure.
#[must_use]
pub fn turn(obs: &Observation) -> (ComputerUseStepRequest, FrameAttachment) {
    turn_after(obs, false)
}

/// [`turn`], telling the model what the last input did to the window.
///
/// An input that ran and left
/// the screen as it was is reported to the model as such (the accounts service's `PrevResult` has no word for
/// "ran, no change", so it goes as a failure with plain words).
pub fn turn_after(obs: &Observation, unchanged: bool) -> (ComputerUseStepRequest, FrameAttachment) {
    let nodes: &[A11yNode] = match &obs.tree {
        TreeObs::Present(snapshot) => &snapshot.nodes,
        TreeObs::Absent(_) => &[],
    };
    let pixels = obs.frame.pixels.clone();
    let tree = match &obs.tree {
        TreeObs::Absent(_) => None,
        TreeObs::Present(snapshot) => Some(TreeText(render(snapshot))),
    };
    let mut request = ComputerUseStepRequest::new(
        obs.step,
        WindowGeometry {
            logical: obs.window.size,
            scale: obs.window.scale,
        },
        FrameImage {
            source: ImageSource::Attached(AttachIndex(0)),
            layout: FrameLayout::Raw {
                format: pixels.format,
                size: pixels.size,
                stride: pixels.stride,
            },
        },
        tree,
    )
    .with_prev(
        obs.prev
            .iter()
            .map(|o| prev_result(o, nodes, unchanged))
            .collect(),
    )
    .with_masked(masked_regions(obs))
    .with_notes(
        obs.notes
            .iter()
            .cloned()
            .chain(obs.prev.iter().filter_map(answer_note))
            .collect(),
    );
    request.cursor = match obs.cursor {
        CursorObs::Unknown => None,
        CursorObs::At(point) => Some(point),
    };
    (request, FrameAttachment(pixels))
}
