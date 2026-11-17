//! `gestures.*`.

use crate::config::convert;
use compositor_core::settings::{CompositorSettings, SwipeFingers};
use serde::{Deserialize, Serialize};
use settings_types::schema::Page;

/// `gestures.*`: the trackpad's multi-finger swipes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, settings_types::SettingsSchema)]
#[serde(default)]
#[settings(file = "compositor/settings.toml", domain = "gestures", page = Page::MouseAndGestures)]
pub struct GesturesSection {
    /// `gestures.spaces`.
    #[settings(
        label = "Swipe between full-screen apps",
        help = "Swipe left or right with this many fingers to move between Spaces.",
        section = "More gestures"
    )]
    pub spaces: SwipeFingers,
    /// `gestures.mission_control`.
    #[settings(
        label = "Mission Control",
        help = "Swipe up with this many fingers to open Mission Control, down to close it.",
        section = "More gestures"
    )]
    pub mission_control: SwipeFingers,
    /// `gestures.natural`.
    #[settings(
        label = "Natural direction",
        help = "The Spaces follow the fingers; off, they move against them.",
        section = "More gestures"
    )]
    pub natural: bool,
}

impl Default for GesturesSection {
    fn default() -> Self {
        convert::gestures_doc(CompositorSettings::defaults().gestures)
    }
}
