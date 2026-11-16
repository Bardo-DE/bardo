//! The run row component.

use super::model::RunRowView;
use crate::components::companion::answer::action::CardActionId;
use crate::components::companion::outcome::model::Outcome;
use crate::components::companion::outcome::view::OutcomeMark;
use dioxus::prelude::*;
use ui_core::word::Word;

/// A run as a row: its goal, the app, the step it is at and its controls. `on_action` hears the
/// control picked. `outcome` is a result the person has not seen: a still dot on the row while
/// it is `Some`, gone when the caller passes `None`.
#[component]
pub fn RunRow(
    view: RunRowView,
    #[props(default)] selection: bool,
    #[props(default)] outcome: Option<Outcome>,
    on_action: EventHandler<CardActionId>,
) -> Element {
    let _ = on_action;
    rsx! {
        div {
            class: "ds-run-row",
            "data-state": view.state.slug(),
            "data-place": view.place.slug(),
            "aria-selected": selection.then_some("true"),
            OutcomeMark { outcome }
        }
    }
}
