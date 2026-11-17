//! The computer-use daemon's configuration: the computer-use settings the person chooses, plus which backend. Parsed
//! once, at the edge. Who may call is not here: it is the process behind a connection, named by
//! `computer-use-callers.toml`.

use account_capability::Tier;
use computer_use_desktop::BackendKind;
use computer_use_run::ComputerUseConfig;
use serde::{Deserialize, Serialize};

/// Which backend the computer-use daemon uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum BackendChoice {
    /// Detect: the fork when the compositor offers `bardo_agent_manager_v1`, otherwise a stock
    /// compositor, otherwise none.
    Detect,
    /// This one, whatever the compositor offers.
    Only(BackendKind),
}

/// Whether the computer-use daemon reads the accessibility tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum A11yChoice {
    /// No tree: every snapshot says there is no accessibility bus, and a run goes on from pixels.
    /// For a person who turned it off.
    Off,
    /// AT-SPI, over the session's accessibility bus. The default: where there is no such bus (a
    /// sandbox), a snapshot says so and the run goes on from pixels. The person's
    /// `computer_use.accessibility` setting still turns reading off for every run.
    #[default]
    Atspi,
}

/// Everything the computer-use daemon reads at start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputerUseDaemonConfig {
    /// The settings (`cua.*`).
    pub settings: ComputerUseConfig,
    /// Which backend.
    pub backend: BackendChoice,
    /// The tier asked of inferd for the computer-use need (`Need::ComputerUse`, class `Screen`).
    pub tier: Tier,
    /// Whether the accessibility tree is read.
    #[serde(default)]
    pub a11y: A11yChoice,
}

impl Default for ComputerUseDaemonConfig {
    fn default() -> Self {
        Self {
            settings: ComputerUseConfig::default(),
            backend: BackendChoice::Detect,
            tier: Tier::Balanced,
            a11y: A11yChoice::default(),
        }
    }
}

/// Why a configuration did not read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("computer-use.toml: {0}")]
pub struct ConfigError(pub String);

impl ComputerUseDaemonConfig {
    /// Reads a configuration from TOML text; a missing file is the caller's decision, a
    /// malformed one is an error here.
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] carrying the parser's message when `text` is not a valid
    /// configuration.
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        toml::from_str(text).map_err(|e| ConfigError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_malformed_configuration_is_an_error() {
        ComputerUseDaemonConfig::from_toml("settings = 3").unwrap_err();
    }
}
