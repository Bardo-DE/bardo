//! `calendar.*`: where the calendar widgets read events, and how the month is drawn (design/20
//! §1.12, which bardo-shell moves into the notification center's widgets as macOS 11 does). All Advanced.

use serde::{Deserialize, Serialize};
use ui_core::word::Word;
use settings_types::schema::Page;

/// The first column of the month grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum FirstWeekday {
    /// ISO 8601, and the week numbers' own week.
    #[default]
    Monday,
    Sunday,
}

/// The calendar widgets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, settings_types::SettingsSchema)]
#[serde(default)]
#[settings(file = "shell/settings.toml", domain = "calendar", page = Page::Notifications)]
pub struct CalendarSettings {
    /// `auto`, `off`, or `;`-separated `.ics` files and directories (parsed by
    /// `shell_model::events::parse_sources`).
    #[settings(
        label = "Calendars",
        help = "Auto finds your calendars; off uses none; or list .ics files and folders, separated by ;.",
        section = "Calendar",
        advanced
    )]
    pub sources: String,
    #[settings(
        label = "First day of the week",
        help = "Which day starts each week.",
        section = "Calendar",
        advanced
    )]
    pub first_weekday: FirstWeekday,
    #[settings(
        label = "Week numbers",
        help = "Shows week numbers beside the month.",
        section = "Calendar",
        advanced
    )]
    /// Whether the month grid has a column of ISO week numbers.
    pub week_numbers: bool,
}

impl Default for CalendarSettings {
    fn default() -> Self {
        CalendarSettings {
            sources: "auto".to_owned(),
            first_weekday: FirstWeekday::Monday,
            week_numbers: false,
        }
    }
}
