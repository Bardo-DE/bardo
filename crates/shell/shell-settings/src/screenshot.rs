//! `screenshot.*`: the floating thumbnail after a screenshot and the folders watched for new ones.
//!
//! The thumbnail is design/20 §1.13. macOS keeps its screenshot shortcuts under Keyboard, hence
//! `Page::KeyboardAndShortcuts`. All Advanced.

use serde::{Deserialize, Serialize};
use ui_settings::schema::Page;

/// Screenshots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ui_settings::SettingsSchema)]
#[serde(default)]
#[settings(
    file = "shell/settings.toml", domain = "screenshot", page = Page::KeyboardAndShortcuts,
    footnote(section = "Screenshots", text = "Auto uses your Pictures folders."),
)]
pub struct ScreenshotSettings {
    #[settings(
        label = "Show floating thumbnail",
        help = "Click it to open, drag it into a document, or swipe it away.",
        section = "Screenshots",
        advanced
    )]
    pub thumbnail: bool,
    /// `auto`, `off`, or `;`-separated folders (`shell_model::screenshots::parse_folders`).
    #[settings(
        label = "Screenshot folders",
        help = "Auto uses your Pictures folders; off watches none; or list folders separated by ;.",
        section = "Screenshots",
        advanced
    )]
    pub folders: String,
}

impl Default for ScreenshotSettings {
    fn default() -> Self {
        ScreenshotSettings {
            thumbnail: true,
            folders: "auto".to_owned(),
        }
    }
}
