//! `pointer.*`.

use crate::config::convert;
use compositor_core::settings::{AccelProfile, CompositorSettings};
use serde::{Deserialize, Serialize};
use settings_types::Fraction;
use settings_types::schema::Page;

/// `pointer.*`: how mice and trackballs behave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, settings_types::SettingsSchema)]
#[serde(default)]
#[settings(file = "compositor/settings.toml", domain = "pointer", page = Page::MouseAndGestures)]
pub struct PointerSection {
    /// `pointer.accel`.
    #[settings(
        label = "Pointer acceleration",
        help = "Adaptive speeds the pointer up with the device's speed; flat moves it in proportion.",
        section = "Mouse"
    )]
    pub accel: AccelProfile,
    /// `pointer.natural_scroll`.
    #[settings(
        label = "Natural scrolling",
        help = "Content follows the fingers.",
        section = "Mouse"
    )]
    pub natural_scroll: bool,
    /// `pointer.tap`.
    #[settings(
        label = "Tap to click",
        help = "A light tap counts as a click.",
        section = "Mouse"
    )]
    pub tap: bool,
    /// `pointer.scroll_factor`.
    #[settings(
        label = "Scroll speed",
        help = "How far a scroll step goes (stored per-mille of the device's own step: 1000 is one step).",
        section = "Mouse",
        range = "100..=4000"
    )]
    pub scroll_factor: Fraction,
    /// `pointer.disable_while_typing`.
    #[settings(
        label = "Ignore while typing",
        help = "The touchpad is ignored while keys are pressed.",
        section = "Mouse"
    )]
    pub disable_while_typing: bool,
}

impl Default for PointerSection {
    fn default() -> Self {
        convert::mouse_doc(CompositorSettings::defaults().pointer.mouse)
    }
}
