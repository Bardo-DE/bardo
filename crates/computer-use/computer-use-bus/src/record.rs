//! What a run writes to the event log. The computer-use daemon stores each record as a memory service
//! `EventBody::Area { area: ComputerUse }` whose `json` is the serde form below and whose kind tag is
//! the variant's slug (`computer_use.run.started`, ...). The actor is
//! `Actor::Companion { session, role: ComputerUse { run } }` ([`computer_use_actor`]).
//!
//! No pixels and no thought text are stored: a thought becomes a digest, a frame a digest and a
//! size. The run's report to its requester is a `provenance::Message`, stored by the router as an
//! `EventBody::Message`; [`ComputerUseRecord::Finished`] links to it.

use crate::action::{ActionResult, TargetedAction};
use crate::budget::RunBudget;
use crate::ids::QuestionId;
use crate::screen::ScreenKind;
use crate::state::ComputerUseOutcome;
use account_core::AppName;
use action_gate_wire::EffectBasis;
use base_ids::Count;
use base_time::UnixSeconds;
use computer_use_action::RunMode;
use computer_use_desktop::{BackendKind, Digest32};
use computer_use_geometry::DeviceSize;
use computer_use_types::FrameDigest;
use computer_use_types::InputReceipt;
use inference_core::{ChargeTotal, ServedBy, StepIndex};
use memory_core::EventRef;
use provenance::{Actor, AgentRole, Effect, Labelled, MessageId, RunId, SessionId, SpaceId};
use serde::{Deserialize, Serialize};

/// A digest of a model's thought: the text is never kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ThoughtDigest(pub Digest32);

/// What is kept of a step's frame. `NotKept` is the default; `Kept` needs `computer_use.screens.keep`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum FrameNote {
    /// A digest and a size.
    NotKept {
        /// The frame's digest.
        digest: FrameDigest,
        /// Its size in device pixels.
        size: DeviceSize,
    },
    /// A thumbnail that expires.
    Kept {
        /// Where the thumbnail is, in the Space's encrypted store.
        thumb: String,
        /// When it goes.
        expires: UnixSeconds,
    },
}

/// Who allowed an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorisedBy {
    /// The router's rulings, without asking.
    Policy,
    /// The person, in a confirmation.
    User,
}

/// How a confirmation was answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmBy {
    /// Hardware input on the compositor's own surface (the fork).
    PhysicalInput,
    /// The shell's question, after the computer-use daemon stopped its input (stock).
    ShellPrompt,
}

/// One record of a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v")]
pub enum ComputerUseRecord {
    /// The run began.
    #[serde(rename = "computer_use.run.started")]
    RunStarted {
        /// The run.
        run: RunId,
        /// The session it acts for.
        session: SessionId,
        /// The conversation it works for (the session that asked for it).
        conversation: SessionId,
        /// What it was asked.
        goal: Labelled<String>,
        /// The app of the leased window.
        app: AppName,
        /// The Space.
        space: SpaceId,
        /// The mode.
        mode: RunMode,
        /// The backend.
        backend: BackendKind,
        /// The limits.
        budget: RunBudget,
    },
    /// inferd said which account and model answers.
    #[serde(rename = "computer_use.run.routed")]
    Routed {
        /// The run.
        run: RunId,
        /// Who answers, and where it ran.
        served_by: ServedBy,
    },
    /// One action, decided and done.
    #[serde(rename = "computer_use.step")]
    Step {
        /// The run.
        run: RunId,
        /// Which step.
        n: StepIndex,
        /// The model's thought, as a digest.
        thought: ThoughtDigest,
        /// The action. Typed text is never kept: a `Type` action stands as a note of its length
        /// and the blake3 digest of its text (`[12 chars, blake3 ...]`).
        action: TargetedAction,
        /// Its effect.
        effect: Effect,
        /// Why that effect.
        basis: EffectBasis,
        /// Who allowed it.
        authorised_by: AuthorisedBy,
        /// How it ended.
        result: ActionResult,
        /// The frame it was decided on.
        frame: FrameNote,
        /// What was injected, if anything.
        input: Option<InputReceipt>,
    },
    /// The look after a step's input: whether the window changed. It follows the `computer_use.step`
    /// records of the actions it judges, and is written only when an input ran and there was a
    /// look before it.
    #[serde(rename = "computer_use.step.seen")]
    Seen {
        /// The run.
        run: RunId,
        /// The step whose look this is (the one after the actions it judges).
        n: StepIndex,
        /// Whether the frame and the tree (when there is one) are the same as before the action.
        /// A run with no earlier look counts as changed.
        unchanged: bool,
    },
    /// The model asked the person something.
    #[serde(rename = "computer_use.asked")]
    Asked {
        /// The run.
        run: RunId,
        /// Which step.
        n: StepIndex,
        /// Which question.
        question_id: QuestionId,
        /// The question, as screen text.
        question: Labelled<String>,
    },
    /// A confirmation was answered yes.
    #[serde(rename = "computer_use.confirmed")]
    Confirmed {
        /// The run.
        run: RunId,
        /// Which step.
        n: StepIndex,
        /// How.
        by: ConfirmBy,
    },
    /// The person took over the window.
    #[serde(rename = "computer_use.taken_over")]
    TakenOver {
        /// The run.
        run: RunId,
        /// Which step.
        n: StepIndex,
    },
    /// The person handed the window back.
    #[serde(rename = "computer_use.handed_back")]
    HandedBack {
        /// The run.
        run: RunId,
        /// Which step.
        n: StepIndex,
        /// What the person did to the window meanwhile, as the log's own addresses (the events
        /// The computer-use daemon's `Memory1` call finds between the take-over and now), oldest first.
        user_events: Vec<EventRef>,
    },
    /// The run ended.
    #[serde(rename = "computer_use.run.finished")]
    Finished {
        /// The run.
        run: RunId,
        /// How.
        outcome: ComputerUseOutcome,
        /// Steps taken.
        steps: Count,
        /// The charges the providers reported, and the replies that reported none.
        charges: ChargeTotal,
        /// The report message the router delivered, if it did.
        report: Option<MessageId>,
    },
    /// A procedure was learned (a later wave).
    #[serde(rename = "computer_use.procedure.learned")]
    ProcedureLearned {
        /// The procedure.
        procedure: String,
        /// The run it came from.
        from: RunId,
    },
    /// The screen check found something before the model was shown the screen. Only the kinds
    /// are kept, never the text. A yes from the person follows as `computer_use.confirmed`; a no ends the
    /// run as `Blocked(ScreenSuspicious(..))`.
    #[serde(rename = "computer_use.screen.suspicious")]
    ScreenSuspicious {
        /// The run.
        run: RunId,
        /// Which step.
        n: StepIndex,
        /// What was found, each kind once.
        kinds: Vec<ScreenKind>,
        /// Whether the run paused and asked the person. Not when they already said to go on for
        /// this window and this kind in this run.
        asked: bool,
    },
}

impl ComputerUseRecord {
    /// The kind tag of this record: the serde tag, as stored.
    #[must_use]
    pub fn kind(&self) -> String {
        crate::slug_of(self).unwrap_or_default()
    }

    /// The run the record belongs to; none for a learned procedure's own row.
    #[must_use]
    pub fn run(&self) -> &RunId {
        match self {
            ComputerUseRecord::RunStarted { run, .. }
            | ComputerUseRecord::Routed { run, .. }
            | ComputerUseRecord::Step { run, .. }
            | ComputerUseRecord::Seen { run, .. }
            | ComputerUseRecord::Asked { run, .. }
            | ComputerUseRecord::Confirmed { run, .. }
            | ComputerUseRecord::TakenOver { run, .. }
            | ComputerUseRecord::HandedBack { run, .. }
            | ComputerUseRecord::Finished { run, .. }
            | ComputerUseRecord::ScreenSuspicious { run, .. } => run,
            ComputerUseRecord::ProcedureLearned { from, .. } => from,
        }
    }
}

/// The actor of every record of a run, as the memory log stores it.
#[must_use]
pub fn computer_use_actor(session: SessionId, run: RunId) -> Actor {
    Actor::Companion {
        session,
        role: AgentRole::ComputerUse { run },
    }
}
