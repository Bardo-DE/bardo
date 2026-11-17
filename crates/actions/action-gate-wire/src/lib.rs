//! The computer-use gate: what `computer_use_daemon` sends to `Intents1.Gate.Check` for every pixel step, and
//! what the router answers. One decision point, one breaker, one audit.
//!
//! Pure and portable: serde only. Sits above `action-core`, whose `CallRefusal` the answer holds,
//! and names `computer-use-action` for the action and the window's trust and mode. `RunMode`,
//! `WindowClass` and `WindowTrust` are `computer-use-action`'s: say `computer_use_action::` for them.

use account_core::AppName;
use action_core::CallRefusal;
use computer_use_action::{ComputerUseAction, RunMode, WindowClass, WindowSpace, WindowTrust};
use provenance::{ActionName, Labelled};
use provenance::{Effect, Label, RunId, SpaceId};
use serde::{Deserialize, Serialize};

/// Where a step's effect class came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum EffectBasis {
    /// The a11y node maps to a typed action.
    NodeTypedAction(ActionName),
    /// The window's class.
    WindowClass(WindowClass),
    /// The default table by action class.
    DefaultTable,
}

/// What an a11y node is, as far as the step touches one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeFacts {
    /// Its role.
    pub role: String,
    /// Its accessible name: somebody else's words.
    pub name: Labelled<String>,
}

/// One pixel step, asking leave to run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputerUseAsk {
    /// The run.
    pub run: RunId,
    /// Which step.
    pub step: u32,
    /// The app in the window.
    pub app: AppName,
    /// How far the window is trusted.
    pub trust: WindowTrust,
    /// How the run reaches it.
    pub mode: RunMode,
    /// The Space.
    pub space: SpaceId,
    /// The action, in window coordinates.
    pub action: ComputerUseAction<WindowSpace>,
    /// The node it touches, if any.
    pub node: Option<NodeFacts>,
    /// The step's effect.
    pub effect: Effect,
    /// Where the effect came from.
    pub basis: EffectBasis,
    /// The screen's label: always `Untrusted`, from `Screen { app }`.
    pub screen: Label,
}

/// The router's answer. Confirmations are resolved inside the router before it answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum GateAnswer {
    /// Run the step.
    Run,
    /// Do not.
    Refused(CallRefusal),
}
