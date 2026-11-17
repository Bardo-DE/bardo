//! The Controls page's buttons with a label of two runs (a quoted-message head) and a leading
//! mark (a dropdown's provider).

use crate::pages::{Section, Specimen};
use dioxus::prelude::*;
use ui_components::components::content::mark_face::MarkFace;
use ui_components::components::content::provider_mark::{MarkProvider, MarkStyle};
use ui_components::components::controls::button_marks::Leading;
use ui_components::components::controls::button_model::Bezel;
use ui_components::prelude::*;
use ui_components::style::tokens::control_size::ControlSize;

/// A provider's inline mark, for a From value.
fn mark(provider: MarkProvider) -> Leading {
    Leading::Mark(rsx! {
        ProviderMark { provider, size: ControlSize::Small, style: MarkStyle::Letter }
    })
}

/// The quoted message's head: who, strong; when, faint.
fn quoted_head() -> TextLine {
    TextLine::Runs(vec![
        TextRun::new("Dana Okafor", RunTone::Strong),
        TextRun::new(" wrote on Tue 22 Sep, 09:41", RunTone::Faint),
    ])
}

#[component]
pub fn LabelRunsAndMarks() -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        Section { title: "Button: a leading mark", note: "leading: Leading::Mark(element) puts a bardo mark before the label: the From dropdown shows the account's provider inside its value. Leading::Glyph(icon) takes a glyph.",
            div { class: "g-row g-row-top",
                Specimen { name: "mark",
                    div { class: "g-row",
                        Button { bezel: Bezel::Inline, label: "poh@acme.example", leading: mark(MarkProvider::Google), trailing: Icon::ChevronDown, onclick: |_| {} }
                        Button { bezel: Bezel::Toolbar, label: "Local folders", leading: mark(MarkProvider::Local), trailing: Icon::ChevronDown, onclick: |_| {} }
                        Button { size: ControlSize::Mini, label: "Pinned", leading: Leading::Glyph(Icon::Pin), onclick: |_| {} }
                    }
                }
            }
        }
        Section { title: "ProviderMark: a face as data", note: "face: Some(MarkFace::new(letter, \"#RRGGBB\")) draws a letter of one or two characters on the colour; bardo picks white or dark ink for the contrast. Two letters are set smaller. A colour that is not #RRGGBB draws the neutral mark.",
            div { class: "g-row g-row-top",
                Specimen { name: "faces",
                    div { class: "g-row",
                        ProviderMark { provider: MarkProvider::Imap, size: ControlSize::Regular, face: Some(MarkFace::new("A", "#D97757")) }
                        ProviderMark { provider: MarkProvider::Imap, size: ControlSize::Regular, face: Some(MarkFace::new("Cx", "#10A37F")) }
                        ProviderMark { provider: MarkProvider::Imap, size: ControlSize::Small, face: Some(MarkFace::new("OR", "#6467F2")) }
                        ProviderMark { provider: MarkProvider::Imap, size: ControlSize::Mini, face: Some(MarkFace::new("LM", "#4B3CC9")) }
                        ProviderMark { provider: MarkProvider::Imap, size: ControlSize::Regular, face: Some(MarkFace::new("K", "#16191E")) }
                        ProviderMark { provider: MarkProvider::Imap, size: ControlSize::Regular, face: Some(MarkFace::new("Q", "teal")) }
                    }
                }
            }
        }
        Section { title: "Button: a label of runs", note: "label takes a Text: a String as before, or runs in their tones drawn inside the label's span (the composer's quoted-message head, who strong and when faint). The button is named by the runs' characters.",
            div { class: "g-row g-row-top",
                Specimen { name: "runs",
                    div { class: "g-row",
                        Button { bezel: Bezel::Inline, label: quoted_head(), trailing: Icon::ChevronDown, shown: open(),
                            onclick: move |_| open.set(!open()) }
                        Button { size: ControlSize::Mini, label: quoted_head(), onclick: |_| {} }
                    }
                }
            }
        }
    }
}
