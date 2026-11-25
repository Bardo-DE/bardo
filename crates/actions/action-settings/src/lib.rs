//! The actions service's settings (design/22 section 3.27): the `agent.*` keys the Settings app writes to
//! `$XDG_CONFIG_HOME/actions/settings.toml`, the schema it draws them from, and the lenient reader
//! the daemons share. A bad value falls back per key and is reported; the file is never fatal.
//! Everything here is a pure function of text (and of what a caller says about the records beside
//! the agents); `action-settings-fs` finds and reads and writes the files, and the live watch is
//! the daemon's (actiond).

mod agents;
mod keys;
mod places;
mod read;
#[cfg(test)]
mod tests;

use action_core::AgentConfig;

pub use agents::{
    AddError, AgentChoice, AgentRecords, AgentRow, Agents, Availability, ChoiceError, Found, KNOWN,
    KnownAgent, ModelState, NewAgent, Offers, Pick, REFRESH_PROGRAM, RefreshRequest, SignInState,
    Source, dirs_of, edit_agent_add, edit_agent_choice, edit_agent_remove, edit_agent_rewind,
    find_known, read_agents, unlisted,
};
pub use places::{
    CloudAccount, CloudAllowed, Floor, Known, KnownPlace, ModelChoice, OwnComputer, PLACES_PATH,
    Place, PlaceKind, PlaceName, PlaceSource, Places, PlacesLoaded, Routed, confirm_turn_on,
    kind_label, no_place_says, read_places, route, write_places,
};
pub use read::read;

/// The schema the actions service ships for its settings (design/22 section 9.2).
pub const SCHEMA: &str = include_str!("../../dist/settings/actions.settings.toml");

/// The file the settings live in, under the configuration directory.
pub const SETTINGS_FILE: &str = "actions/settings.toml";

/// The reviewers' own timeouts when the router enforces its deadline: long ones, so the router's
/// live deadline is the one that fires.
pub const REVIEW_CEILING: action_core::ReviewTimeouts = action_core::ReviewTimeouts {
    quick: action_vocab::Millis(10_000),
    deliberate: action_vocab::Millis(30_000),
    second: action_vocab::Millis(30_000),
};

/// Every value of the file, typed: what the router and the companion read, and whether the MCP and
/// ACP edges are on.
///
/// The file stores the three switches as TOML `true` / `false`. All three are off by default: an
/// edge that was never switched on serves nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentSettings {
    /// The proposed values, with the person's choices in them.
    pub agent: AgentConfig,
    /// `agent.mcp.expose`: whether external MCP clients may use the registry's offered actions.
    pub expose: bool,
    /// `agent.acp.expose`: whether a code editor may drive the companion over ACP.
    pub acp: bool,
    /// `agent.acp.agents`: whether the actions service may run external coding agents as session backends.
    pub agents: bool,
}

impl AgentSettings {
    /// The settings of a machine with no file: `agent`'s values, the edge off.
    #[must_use]
    pub fn over(agent: AgentConfig) -> Self {
        Self {
            agent,
            expose: false,
            acp: false,
            agents: false,
        }
    }
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self::over(AgentConfig::default())
    }
}
