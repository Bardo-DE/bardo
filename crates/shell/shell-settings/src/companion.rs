//! `companion.*`: the summon and the confirmation (design/22 §3.30).
//!
//! The double tap's window and how long a focused app has to take a summon are the machines' own
//! constants; the service reads [`params`] once per load.

use std::time::Duration;

use base_words::Word;
use serde::{Deserialize, Serialize};
use settings_types::schema::Page;
use shell_model::companion::{CompanionParams, SummonParams};

pub use shell_model::companion::InField;

/// `companion.summon`: what summons the companion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum SummonKey {
    /// Pressing the command key twice.
    #[default]
    DoubleTapCommand,
    /// No key summons it.
    Off,
}

/// `companion.notify`: when the companion tells the person it needs them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum NotifyWhen {
    /// When a task waits for the person.
    #[default]
    NeedsYou,
    /// Never.
    Never,
}

/// The companion's summon and confirmation.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, settings_types::SettingsSchema,
)]
#[serde(default)]
#[settings(
    file = "shell/settings.toml", domain = "companion", page = Page::Intelligence,
    footnote(section = "Opening", text = "Press the command key twice to ask, or hold it to talk."),
)]
pub struct CompanionSettings {
    #[settings(
        label = "Press ⌘ twice to ask",
        help = "Opens the assistant when you press ⌘ twice.",
        section = "Opening",
        note = "Press the command key twice in a row."
    )]
    pub summon: SummonKey,
    #[settings(
        label = "In a text field",
        help = "Makes the field the prompt, or opens the launcher.",
        section = "Opening"
    )]
    pub in_field: InField,
    #[settings(
        label = "Notify when it needs you",
        help = "Posts a notification when a task is waiting for you.",
        section = "Alerts"
    )]
    pub notify: NotifyWhen,
}

/// What the companion service's machines run on for `settings`: the double tap's window and
/// the app's response time are the machines' own constants, the summon's place the key's.
#[must_use]
pub fn params(settings: &CompanionSettings) -> CompanionParams {
    CompanionParams {
        summon: SummonParams {
            in_field: settings.in_field,
            ..SummonParams::default()
        },
        ..CompanionParams::default()
    }
}

/// How long a question's buttons answer nothing after it is drawn. The compositor refuses an
/// answer that comes sooner than 500 ms after its question; 600 leaves a margin.
pub const CONFIRM_ARM: Duration = Duration::from_millis(600);

/// [`CONFIRM_ARM`], for the surfaces that arm their buttons.
#[must_use]
pub fn confirm_arm() -> Duration {
    CONFIRM_ARM
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_field_reaches_the_summon_machine() {
        let settings = CompanionSettings {
            in_field: InField::Launcher,
            ..CompanionSettings::default()
        };
        assert_eq!(params(&settings).summon.in_field, InField::Launcher);
    }
}
