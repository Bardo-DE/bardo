//! `spaces.*`.

use crate::config::convert;
use compositor_core::settings::CompositorSettings;
use serde::{Deserialize, Serialize};
use settings_types::schema::Page;

/// `spaces.*`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, settings_types::SettingsSchema)]
#[serde(default)]
#[settings(file = "compositor/settings.toml", domain = "spaces", page = Page::Spaces)]
pub struct SpacesSection {
    /// `spaces.per_display`. Held but not yet a behaviour (every display has its own Spaces), so
    /// The Settings app draws no row for it.
    #[settings(skip)]
    pub per_display: bool,
    /// `spaces.auto_rearrange`. Held but not yet a behaviour (open D4), so the Settings app draws
    /// no row for it.
    #[settings(skip)]
    pub auto_rearrange: bool,
    /// `spaces.switch_on_activate`.
    #[settings(
        label = "When switching to an app, switch to a Space with its open windows",
        help = "Opening or switching to an app shows the Space its window is on, and the window takes the keyboard. When off, you stay on this Space, and a window on another Space asks for your attention in the Dock instead.",
        section = "Spaces"
    )]
    pub switch_on_activate: bool,
}

impl Default for SpacesSection {
    fn default() -> Self {
        convert::spaces_doc(CompositorSettings::defaults().spaces)
    }
}
