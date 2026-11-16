//! The search card: the field and its results as one surface, Spotlight's panel. Drawn where it
//! stands, or floated at the window level from a `CardPlace`, in which case the popover is the
//! surface and the card adds only its layout.

use crate::components::menus::search::model::CardPlace;
use crate::components::overlays::popover::Popover;
use dioxus::prelude::*;
use ui_core::geometry::units::Px;
use ui_core::vocab::Dismiss;

/// The card around `field`, with `results` under a hairline when there are any.
pub(crate) fn card(
    field: Element,
    results: Option<Element>,
    place: Option<CardPlace>,
    ondismiss: EventHandler<()>,
) -> Element {
    let inside = rsx! {
        {field}
        if let Some(results) = results {
            div { class: "ds-search-card-rows", {results} }
        }
    };
    match place {
        Some(CardPlace { anchor, placement }) => rsx! {
            Popover {
                anchor,
                placement,
                gap: Px(0.0),
                dismiss: Dismiss::Transient,
                onclose: move |()| ondismiss.call(()),
                div { class: "ds-search-card", {inside} }
            }
        },
        None => rsx! {
            div { class: "ds-search-card", "data-surface": "own", {inside} }
        },
    }
}
