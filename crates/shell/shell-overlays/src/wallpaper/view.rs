//! The wallpaper's render: an `Ds` root over two cross-fading image layers, and the desktop's
//! menu at a press of the secondary button.

use dioxus::prelude::*;
use layer_shell_host::OutputInfo;
use shell_model::ids::OutputName;
use ui_components::prelude::*;

use super::io::use_crossfade;
use super::menu::{secondary_press, use_desktop_menu};
use super::model::url_for;
use shell_services::WallpaperService;
use shell_ui_kit::hooks::{use_appearance, use_service};
use shell_ui_kit::menu::use_popup_look;
use shell_ui_kit::root::use_root_style;

/// The wallpaper's own stylesheet (lint-clean under `Profile::Strict`, `crates/sill/tests/lint.rs`).
pub const WALLPAPER_CSS: &str = include_str!("style.css");

/// The wallpaper on `output`: an `Ds` root over two cross-fading image layers showing the
/// cached file for the resolved scheme (`ui_components::prelude::resolve`, the same call `Ds` makes internally, so
/// the two never disagree about which scheme is current).
pub(super) fn wallpaper(output: OutputInfo) -> Element {
    let environment = use_appearance()();
    let wallpapers = use_service::<WallpaperService>()();
    let on = output.name.map(OutputName);
    let appearance = environment.settings.appearance.appearance();
    let resolved = resolve(appearance, SpaceLook::default().theme, environment.system);
    let url = url_for(&wallpapers, on.as_ref(), resolved.scheme);
    let layers = use_crossfade(url);
    let root = use_root_style();
    let menu = use_desktop_menu(use_popup_look());
    rsx! {
        Ds {
            sheet: Some(shell_kit::stylesheet()),
            appearance,
            system: environment.system,
            material: Material::Widget,
            extent: RootExtent::Viewport,
            blur: false,
            surface: root.surface,
            styles: root.styles,
            custom_styles: root.custom_styles,
            AppStyle { css: WALLPAPER_CSS }
            div {
                class: "sill-wallpaper",
                onmousedown: move |event| secondary_press(&event, menu),
                for (slot , back , image) in layers {
                    div {
                        key: "{slot}",
                        class: "sill-wallpaper-layer",
                        "data-layer": back,
                        if let Some(url) = image {
                            img { src: "{url}", alt: "" }
                        }
                    }
                }
            }
        }
    }
}
