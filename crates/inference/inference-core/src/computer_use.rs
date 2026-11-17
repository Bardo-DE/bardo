//! The computer-use step: the contract between the computer-use daemon and inferd. Every point is in
//! `WindowSpace` (logical window pixels); inferd's `computer-use-session` maps to and from the model's
//! own space and drops anything out of frame.

use crate::error::ModelError;
use crate::request::ImageSource;
use account_capability::ComputerUseEnv;
use computer_use_action::{
    ComputerUseAction, DeviceSize, PixelFormat, Point, Scale120, Size, WindowSpace,
};
use serde::{Deserialize, Serialize};
use std::fmt;

pub use computer_use_parse::DropReason;
pub use computer_use_session::{MaskedRegions, StepIndex, TreeText};
pub use model_vision_prep::MediaType;

/// Opens a computer-use run: the goal and the planner's hints. Later `ComputerUseStep` requests on the
/// same session continue it; the session is pinned to one model so its history stays valid.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputerUseBegin {
    /// What the person asked for.
    pub goal: String,
    /// Short typed hints from the planner.
    pub hints: Vec<String>,
    /// What is operated.
    pub env: ComputerUseEnv,
}

impl ComputerUseBegin {
    /// A run toward `goal` in `env`, with no hints.
    #[must_use]
    pub fn new(goal: String, env: ComputerUseEnv) -> Self {
        Self {
            goal,
            hints: Vec::new(),
            env,
        }
    }

    /// The same run with these hints from the planner.
    #[must_use]
    pub fn with_hints(mut self, hints: Vec<String>) -> Self {
        self.hints = hints;
        self
    }
}

// The goal is what the person asked for: Debug shows sizes only.
impl fmt::Debug for ComputerUseBegin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ComputerUseBegin(<goal {} chars, {} hints>, {:?})",
            self.goal.chars().count(),
            self.hints.len(),
            self.env
        )
    }
}

/// One step: what the window shows now and what happened to the last actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputerUseStepRequest {
    /// Which step.
    pub step: StepIndex,
    /// The window's size and scale.
    pub window: WindowGeometry,
    /// The frame.
    pub frame: FrameImage,
    /// Where the pointer is, if known.
    pub cursor: Option<Point<WindowSpace>>,
    /// What became of the previous step's actions, in order.
    pub prev: Vec<PrevResult>,
    /// How many regions were blacked out of the frame.
    pub masked: MaskedRegions,
    /// The window's accessibility text, when there is one.
    pub tree: Option<TreeText>,
    /// What was said to the run since the last step, oldest first (a message that arrived
    /// while it was acting); empty when nothing was.
    pub notes: Vec<StepNote>,
}

impl ComputerUseStepRequest {
    /// Step `step`: what the window shows (`frame`, in `window`) and its accessibility text. No
    /// pointer position, no earlier results, no masked regions and no notes until the `with_*`
    /// methods say so.
    #[must_use]
    pub fn new(
        step: StepIndex,
        window: WindowGeometry,
        frame: FrameImage,
        tree: Option<TreeText>,
    ) -> Self {
        Self {
            step,
            window,
            frame,
            cursor: None,
            prev: Vec::new(),
            masked: MaskedRegions(0),
            tree,
            notes: Vec::new(),
        }
    }

    /// The same step with the pointer at `cursor`.
    #[must_use]
    pub fn with_cursor(mut self, cursor: Point<WindowSpace>) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// The same step with what became of the previous step's actions.
    #[must_use]
    pub fn with_prev(mut self, prev: Vec<PrevResult>) -> Self {
        self.prev = prev;
        self
    }

    /// The same step with this many regions blacked out of the frame.
    #[must_use]
    pub fn with_masked(mut self, masked: MaskedRegions) -> Self {
        self.masked = masked;
        self
    }

    /// The same step with what was said to the run since the last one.
    #[must_use]
    pub fn with_notes(mut self, notes: Vec<StepNote>) -> Self {
        self.notes = notes;
        self
    }
}

/// One message that reached a running computer-use task mid-run.
///
/// A note is input, never
/// authority (the rule of `provenance::Message`): computer-use-session fences an agent's note as untrusted
/// text, and inferd only forwards it. `Debug` shows the length only.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepNote {
    /// Who said it.
    pub from: NoteFrom,
    /// What they said.
    pub text: String,
}

impl fmt::Debug for StepNote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StepNote({:?}, <{} bytes>)", self.from, self.text.len())
    }
}

/// Who a [`StepNote`] is from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteFrom {
    /// The person: an instruction about the task.
    Person,
    /// Another agent (the companion, a worker): information, to be weighed.
    Agent,
}

/// The leased window as the model must see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowGeometry {
    /// Its logical size.
    pub logical: Size<WindowSpace>,
    /// Its scale: 120 is 100%.
    pub scale: Scale120,
}

/// A frame of the window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameImage {
    /// Where its bytes are (a memfd, for raw frames).
    pub source: ImageSource,
    /// How to read them.
    pub layout: FrameLayout,
}

/// How a frame's bytes are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum FrameLayout {
    /// Raw pixels, preferred: inferd resizes once and encodes once.
    Raw {
        /// The pixel format.
        format: PixelFormat,
        /// Width and height in device pixels.
        size: DeviceSize,
        /// Bytes per row.
        stride: u32,
    },
    /// An encoded image.
    Encoded(MediaType),
}

/// What became of one action of the previous step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum PrevResult {
    /// It ran.
    Done,
    /// The gate or a local check refused it, with the reason shown to the model.
    Refused(String),
    /// An earlier action failed, so this one was not tried.
    NotRun,
    /// It was tried and failed.
    Failed(String),
    /// The person declined it.
    UserDeclined,
    /// The person did it themselves.
    UserActed,
}

/// The model's answer to one step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputerUseStepReply {
    /// Its reasoning, when it gave any.
    pub thought: Option<String>,
    /// The actions to try, in order, in window space.
    pub actions: Vec<ComputerUseAction<WindowSpace>>,
    /// What was read but not usable.
    pub dropped: Vec<DroppedAction>,
    /// Vendor hints. They only ever add asks.
    pub safety: Vec<SafetyHint>,
}

impl ComputerUseStepReply {
    /// A reply proposing these actions: no reasoning, nothing dropped and no vendor hints until
    /// the `with_*` methods say so.
    #[must_use]
    pub fn new(actions: Vec<ComputerUseAction<WindowSpace>>) -> Self {
        Self {
            thought: None,
            actions,
            dropped: Vec::new(),
            safety: Vec::new(),
        }
    }

    /// The same reply with the model's reasoning.
    #[must_use]
    pub fn with_thought(mut self, thought: String) -> Self {
        self.thought = Some(thought);
        self
    }

    /// The same reply with what was read but not usable.
    #[must_use]
    pub fn with_dropped(mut self, dropped: Vec<DroppedAction>) -> Self {
        self.dropped = dropped;
        self
    }

    /// The same reply with these vendor hints.
    #[must_use]
    pub fn with_safety(mut self, safety: Vec<SafetyHint>) -> Self {
        self.safety = safety;
        self
    }
}

/// Something the model said that became no action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroppedAction {
    /// The verb it used (at most 64 characters).
    pub verb: String,
    /// Why it was dropped.
    pub reason: DropReason,
}

/// A vendor's hint about a step. It can add a confirmation or a block; it cannot remove one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum SafetyHint {
    /// Ask the person first, for this reason.
    RequireConfirmation(String),
    /// Do not do it, for this reason.
    Blocked(String),
}

/// Why a step produced no reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ComputerUseStepFailure {
    /// The reply could not be parsed, even after the repair attempt.
    Unparseable,
    /// The model call failed.
    ModelFailed(ModelError),
}
