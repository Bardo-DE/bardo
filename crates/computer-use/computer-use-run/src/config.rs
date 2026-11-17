//! The proposed values the specs name, as one typed configuration with its defaults. This crate
//! never reads settings: `computer-use-host` holds the key table and the schema, and the computer-use daemon receives a
//! [`ComputerUseConfig`].
//!
//! `computer_use.budget.denials` is not here: the router's breaker (`agent.breaker.*`) counts denials.

use computer_use_a11y_tree::NodeBudget;
use computer_use_bus::RunBudget;
use computer_use_desktop::Ms;
use serde::{Deserialize, Serialize};

/// `computer_use.default_mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultMode {
    /// The agent workspace where the backend fully supports it, otherwise in place.
    Auto,
    /// In place.
    InPlace,
    /// The agent workspace.
    AgentWorkspace,
}

/// A number of days.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Days(pub u32);

/// How long to wait for the screen to settle after an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SettleLimits {
    /// No change for this long means settled.
    pub quiet: Ms,
    /// Give up after this long.
    pub timeout: Ms,
}

/// How long to wait for the screen to settle after an action: quiet for 250 ms, at most 3 s.
pub const SETTLE: SettleLimits = SettleLimits {
    quiet: Ms(250),
    timeout: Ms(3000),
};

/// The longest one `wait` may last: 60 s, the most the action can ask.
pub const WAIT_MAX: Ms = Ms(60_000);

/// How long a run may keep waiting, `wait` after `wait`, before an unchanged screen counts as
/// stalled again: ten minutes.
pub const WAIT_TOTAL: Ms = Ms(600_000);

/// How many tree nodes a snapshot keeps.
pub const TREE_MAX_NODES: NodeBudget = NodeBudget(400);

/// Every proposed value of computer use in one place.
#[expect(
    clippy::struct_excessive_bools,
    reason = "one field per boolean `cua.*` settings key; they are independent switches"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputerUseConfig {
    /// `computer_use.enabled`: off until the person turns it on, and then still needs the per-app
    /// consent. Off, `ComputerUse1.Start` answers `Disabled` and the `org.bardo.ComputerUse` action is hidden.
    pub enabled: bool,
    /// `computer_use.default_mode`.
    pub default_mode: DefaultMode,
    /// `computer_use.budget.*`.
    pub budget: RunBudget,
    /// `computer_use.autopause`: whether physical input pauses a run on a shared seat.
    pub autopause: bool,
    /// `computer_use.accessibility` (on): whether a run reads the controls (buttons, fields) of the
    /// window it is using, as the app describes them. Only that window's; with it off a run
    /// goes from pixels.
    pub accessibility: bool,
    /// `computer_use.screens.keep`: whether thumbnails of steps are kept in the Space's encrypted store.
    pub keep_screens: bool,
    /// `computer_use.screens.keep_days` (7).
    pub keep_days: Days,
}

impl Default for ComputerUseConfig {
    fn default() -> Self {
        ComputerUseConfig {
            enabled: false,
            default_mode: DefaultMode::Auto,
            budget: RunBudget::PROPOSED,
            autopause: true,
            accessibility: true,
            keep_screens: false,
            keep_days: Days(7),
        }
    }
}
