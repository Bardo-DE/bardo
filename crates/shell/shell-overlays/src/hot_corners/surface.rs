//! The hot corners as a [`Surface`]: a slot per corner with a binding, shown at
//! once.

use dioxus::prelude::Element;
use layer_shell_host::{LayerConfig, OutputInfo};
use shell_settings::ShellFile;

use super::layer::layer;
use super::model::active_corners;
use super::view::hot_corner;
use shell_ui_kit::links::{Corner, CornerLink, Links};
use shell_ui_kit::surface::{Place, Presence, Surface};

/// The transparent squares in the corners of an output that fire an action on a dwell.
#[derive(Debug)]
pub enum HotCorners {}

impl Surface for HotCorners {
    const NAME: &'static str = "hot-corner";
    type Link = CornerLink;
    type Slot = Corner;
    const LAYER_ORDER: u8 = 210;
    const PRESENCE: Presence = Presence::Always;

    fn link(links: &Links) -> CornerLink {
        links.corners.clone()
    }

    fn slots(place: &Place) -> Vec<Corner> {
        active_corners(&place.settings.hot_corners)
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, slot: Corner) -> LayerConfig {
        layer(output, slot)
    }

    fn root(output: OutputInfo, slot: Corner) -> Element {
        hot_corner(&output, slot)
    }
}
