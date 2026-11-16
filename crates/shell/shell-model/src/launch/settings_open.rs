//! Opening Settings at a place: the link (Settings' own target type, which every
//! entrance hands over) and the activation token that lets Settings take focus.

use serde::{Deserialize, Serialize};
use shell_launcher::SettingsTarget;
use shell_launcher::settings_target::page;

/// A request to open Settings at `link`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsOpen {
    #[serde(with = "shell_launcher::settings_target::text")]
    pub link: SettingsTarget,
    /// `None` when the compositor refused one.
    pub token: Option<String>,
}

impl SettingsOpen {
    /// A request with no token.
    #[must_use]
    pub fn at(link: SettingsTarget) -> SettingsOpen {
        SettingsOpen { link, token: None }
    }

    /// This request carrying `token`.
    #[must_use]
    pub fn with_token(self, token: Option<String>) -> SettingsOpen {
        SettingsOpen { token, ..self }
    }
}

/// The picker for the assistant's model: opened when a turn needs a model and none is set.
#[must_use]
pub fn assistant_model() -> SettingsTarget {
    page("assistant-model")
}
