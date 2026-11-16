//! `SectionHeader` and Disclosure: the one header look with and without a value, an action or a
//! collapse, and the triangle that opens a body (design/30 sections 2.1 and 2.6).

use crate::pages::{Section, Specimen};
use dioxus::prelude::*;
use ui_components::components::controls::disclosure::Disclosure;
use ui_components::prelude::*;
use ui_components::style::tokens::control_size::ControlSize;

/// The headers and the disclosures.
#[component]
pub fn HeaderGallery() -> Element {
    let mut source = use_signal(|| true);
    rsx! {
        Section {
            title: "SectionHeader",
            note: "One look. A value sits at the end, an action after it (a palette's cursor can rest on it), and in a source list the title is a button with a triangle.",
            div { class: "g-row g-row-top",
                Specimen { name: "Title", div { class: "g-list", style: "width:240px", SectionHeader { title: "Recent" } } }
                Specimen { name: "Value", div { class: "g-list", style: "width:240px", SectionHeader { title: "Grain", value: "12".to_string() } } }
                Specimen { name: "Action", div { class: "g-list", style: "width:240px", SectionHeader { title: "Files", actions: vec![HeaderAction::new("Show More", EventHandler::new(|()| {}))] } } }
                Specimen { name: "Action selected", div { class: "g-list", style: "width:240px",
                    SectionHeader { title: "Files", actions: vec![HeaderAction { selection: true, ..HeaderAction::new("Show More", EventHandler::new(|()| {})) }] }
                } }
                Specimen { name: "Two actions", div { class: "g-list", style: "width:240px",
                    SectionHeader { title: "Labels", actions: vec![HeaderAction::new("Select All", EventHandler::new(|()| {})), HeaderAction::new("Clear", EventHandler::new(|()| {}))] }
                } }
                Specimen { name: "Collapsible", div { class: "g-list", style: "width:240px",
                    SectionHeader { title: "Favourites", collapse: Some((source(), EventHandler::new(move |to| source.set(to)))) }
                } }
            }
        }
        Disclosures {}
    }
}

/// The triangle at each size, open and closed, busy, disabled, with a body.
#[component]
fn Disclosures() -> Element {
    let mut open = use_signal(|| true);
    let mut closed = use_signal(|| false);
    rsx! {
        Section {
            title: "Disclosure",
            note: "The triangle turns a quarter over --t-quick; the body's height and opacity follow over --t-move from the height it measured, reversing mid-move if pressed again. Under Reduced motion both change at once.",
            div { class: "g-row g-row-top",
                Specimen { name: "Open", code: "shown: Visible".to_string(),
                    Disclosure { shown: open(), on_toggle: move |to| open.set(to), label: Some("Advanced".to_string()),
                        p { class: "g-note", "Three lines of settings that were folded away." }
                        p { class: "g-note", "The body keeps its own height." }
                    }
                }
                Specimen { name: "Closed", code: "shown: Hidden".to_string(),
                    Disclosure { shown: closed(), on_toggle: move |to| closed.set(to), label: Some("Details".to_string()),
                        p { class: "g-note", "Opens from its measured height." }
                    }
                }
                Specimen { name: "Disabled",
                    Disclosure { shown: false, on_toggle: |_| {}, label: Some("Locked".to_string()), availability: Availability::Disabled }
                }
                Specimen { name: "Busy",
                    Disclosure { shown: false, on_toggle: |_| {}, label: Some("Loading".to_string()), availability: Availability::Busy }
                }
                Specimen { name: "Sizes",
                    div { class: "g-col",
                        for size in [ControlSize::Mini, ControlSize::Small, ControlSize::Regular, ControlSize::Large] {
                            Disclosure { key: "{size:?}", shown: false, on_toggle: |_| {}, label: Some(format!("{size:?}")), size }
                        }
                    }
                }
            }
        }
    }
}
