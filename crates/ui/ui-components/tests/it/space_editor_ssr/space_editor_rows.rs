//! The mail app's Space editor rows: the name field, the Motion row and the readout per scheme
//! (both for a System Space, one for a Dark one), an unnamed Space's placeholder, and the Motion
//! row alone. Each is a named state under `space_editor`.

use dioxus::prelude::*;
use ui_components::components::app::space_editor::DotIndex;
use ui_components::components::app::space_editor::rows::{MeasuredIn, MotionChoice};
use ui_components::prelude::*;
use ui_style::space::presets::PRESETS;

/// Preset `index` as a Space's look, in `theme`.
fn look(index: usize, theme: Theme) -> SpaceLook {
    SpaceLook {
        dots: PRESETS[index].dots.to_vec(),
        grain: ui_components::prelude::Grain(35),
        theme,
    }
}

/// The editor with every mail-app row switched on, for a Space whose theme is `theme`.
fn editor_rows(theme: Theme) -> Element {
    rsx! {
        SpaceEditor {
            look: look(0, theme),
            scheme: Scheme::Light,
            active_dot: DotIndex(0),
            onchange: |_| {},
            name: "Work".to_string(),
            on_rename: EventHandler::new(|_: String| {}),
            motion: MotionChoice { level: Motion::Reduced, on_motion: EventHandler::new(|_: Motion| {}) },
            measured: MeasuredIn::EachScheme,
        }
    }
}

/// One state and its name.
pub struct Case {
    pub name: &'static str,
    pub make: fn() -> Element,
}

pub const CASES: &[Case] = &[
    Case {
        name: "space_editor/rows-system",
        make: || editor_rows(Theme::System),
    },
    Case {
        name: "space_editor/rows-dark",
        make: || editor_rows(Theme::Dark),
    },
    Case {
        name: "space_editor/rename-unnamed",
        make: || rsx! { SpaceEditor { look: look(2, Theme::Light), scheme: Scheme::Light, active_dot: DotIndex(0), onchange: |_| {}, on_rename: EventHandler::new(|_: String| {}) } },
    },
    Case {
        name: "space_editor/motion-row",
        make: || {
            rsx! {
                SpaceEditor {
                    look: SpaceLook::default(),
                    scheme: Scheme::Light,
                    active_dot: DotIndex(0),
                    onchange: |_| {},
                    motion: MotionChoice { level: Motion::Reduced, on_motion: EventHandler::new(|_: Motion| {}) },
                }
            }
        },
    },
];
