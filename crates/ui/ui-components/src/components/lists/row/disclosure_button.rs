//! A branch row's disclosure triangle as a control: it has a name ("Expand" or "Collapse"), a
//! tip with it, and Return or Space press it.

use crate::components::content::tip_text::TipText;
use crate::components::content::title_tip::use_tip;
use crate::components::controls::disclosure::indicator;
use crate::components::controls::press::{ActivationKeys, activates};
use crate::focus::click::kept_click;
use dioxus::prelude::*;
use ui_core::vocab::aria_word;

/// The name and tip of the triangle for a row that is `shown`: what a press will do.
fn name(shown: bool) -> &'static str {
    if shown { "Collapse" } else { "Expand" }
}

/// The triangle of a branch row; a press asks `on_toggle` for the other state. It never reaches
/// the row.
#[component]
pub(crate) fn RowDisclosure(shown: bool, on_toggle: Option<EventHandler<bool>>) -> Element {
    let label = name(shown);
    let tip = use_tip(Some(TipText::new(label)));
    let toggle = move || {
        if let Some(on_toggle) = on_toggle {
            on_toggle.call(!shown);
        }
    };
    rsx! {
        span {
            class: "ds-row-disclosure",
            "data-outline": "branch",
            role: "button",
            tabindex: "0",
            title: tip.native(),
            "aria-label": label,
            "aria-expanded": aria_word(shown),
            onclick: move |event| {
                event.stop_propagation();
                event.prevent_default();
                toggle();
                kept_click(&event);
            },
            onkeydown: move |event| {
                if activates(&event, ActivationKeys::ReturnAndSpace) {
                    event.stop_propagation();
                    event.prevent_default();
                    toggle();
                }
            },
            onmousedown: {
                let tip = tip.clone();
                move |event| {
                    event.stop_propagation();
                    tip.press();
                }
            },
            onmouseover: {
                let tip = tip.clone();
                move |event| tip.over(&event)
            },
            onmouseleave: {
                let tip = tip.clone();
                move |_| tip.out()
            },
            onmounted: {
                let tip = tip.clone();
                move |event| tip.mounted(&event)
            },
            {indicator(shown)}
        }
        {tip.surface()}
    }
}
