//! The dim's render: Bardo's `IdleDim` alone in a transparent viewport root.

use dioxus::prelude::*;
use layer_shell_types::OutputInfo;
use shell_kit::idle_dim::IdleDim;
use ui_components::prelude::*;

use super::io::use_dim;
use shell_ui_kit::hooks::use_appearance;
use shell_ui_kit::root::use_root_style;

/// The dim on `output`.
pub(super) fn idle_dim(output: &OutputInfo) -> Element {
    let _ = output;
    let scene = use_dim();
    let environment = use_appearance()();
    let root = use_root_style();
    rsx! {
        Ds {
            sheet: Some(shell_kit::stylesheet()),
            appearance: environment.settings.appearance.appearance(),
            system: environment.system,
            material: Material::Sheet,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            surface: root.surface,
            styles: root.styles,
            custom_styles: root.custom_styles,
            IdleDim { level: scene.level, phase: scene.phase }
        }
    }
}
