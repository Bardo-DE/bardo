//! The indicator's render: one transparent Osd root (design/20 §1.7) holding quire's `Osd` card.
//! The card paints its own tinted frame, owns its own entrance and exit fade, and calls
//! `on_hidden` once the exit has settled, so `io.rs` can unmap the surface then; bardo-shell's own
//! hold machine (`step.rs`) only decides when the card is shown.

use dioxus::prelude::*;
use layer_shell_host::{OutputInfo, Visibility};
use shell_kit::osd::OsdPosition;
use shell_kit::prelude::*;
use shell_model::ids::OutputName;
use ui_components::components::controls::level_indicator::LevelStyle;
use ui_components::prelude::*;
use ui_components::root::common::Common;

use super::io::{times, use_osd};
use super::layer::{CARD_ID, layer};
use super::reading::{card_level, level_of, title};
use super::tokens::OsdTokens;
use shell_services::{DisplaysService, VolumeService};
use shell_ui_kit::hooks::{use_follow_place, use_service, use_surface_tokens};
use shell_ui_kit::links::{OsdKind, OsdLink};
use shell_ui_kit::root::SurfaceRoot;

/// The indicator's stylesheet (lint-clean under `Profile::Strict`, `crates/shell/shell-app/tests/lint.rs`).
pub const OSD_CSS: &str = include_str!("style.css");

/// The level's style: the bezel's sixteen squares, as the Mac's volume and brightness keys draw
/// them (quire design/29 section 16); the capsule was the iOS pill. Not a setting: the OSD has no
/// look key, and this is the one place.
const LOOK: LevelStyle = LevelStyle::Discrete;

/// The indicator on `output`.
pub(super) fn osd(output: OutputInfo) -> Element {
    let on = output.name.clone().map(OutputName);
    let resolved = use_surface_tokens::<OsdTokens>();
    let place = layer(&output, &resolved.get());
    use_follow_place(place.size, place.margin);
    let link = use_context::<OsdLink>();
    let drawn = use_osd(&link, output.id, times());
    let volume = use_service::<VolumeService>()();
    let displays = use_service::<DisplaysService>()();
    let kind = drawn.change.as_ref().map_or(OsdKind::Volume, |c| c.kind);
    let level = drawn
        .change
        .as_ref()
        .and_then(|c| level_of(c.kind, c.level, &volume, &displays, on.as_ref()));
    let shown = match drawn.phase.visible() {
        Visibility::Shown => true,
        Visibility::Hidden => false,
    };
    rsx! {
        SurfaceRoot {
            output,
            material: Material::Osd,
            chrome: Some(RootChrome::Transparent),
            AppStyle { css: OSD_CSS }
            div { onmounted: move |event| resolved.mounted(event),
                Osd {
                    shown,
                    label: Some(title(kind, &volume, &displays, on.as_ref())),
                    level: Some(card_level(kind, level)),
                    position: OsdPosition::TopRight,
                    style: LOOK,
                    common: Common { id: Some(CARD_ID.to_owned()), ..Common::default() },
                    on_hidden: drawn.on_hidden,
                }
            }
        }
    }
}
