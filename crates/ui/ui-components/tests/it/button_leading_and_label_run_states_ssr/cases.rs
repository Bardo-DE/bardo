//! The mail-app states, as data: the name each renders under and how to make it.

use dioxus::prelude::*;
use ui_components::components::content::provider_mark::{MarkProvider, MarkStyle};
use ui_components::components::controls::button_marks::Leading;
use ui_components::components::controls::button_model::Bezel;
use ui_components::prelude::*;
use ui_components::root::pass_through::{DataAttr, DataName};
use ui_core::vocab::RowState;
use ui_style::tokens::control_size::ControlSize;

/// One state and its name.
pub struct Case {
    pub name: &'static str,
    pub make: fn() -> Element,
}

/// The composer's quoted-message head: who in the strong tone, when in the faint one.
fn quoted_head() -> TextLine {
    TextLine::Runs(vec![
        TextRun::new("Dana Okafor", RunTone::Strong),
        TextRun::new(" wrote on Tue 22 Sep, 09:41", RunTone::Faint),
    ])
}

/// The Archive place during a drag, in `drop` state.
fn archive(drop: DropState) -> Element {
    let place = DataName::parse("place")
        .map_or_default(|name| vec![DataAttr::new(name, "archive")]);
    rsx! {
        Row {
            title: "Archive",
            leading: RowLeading::Icon(Icon::Archive),
            state: RowState { drop, ..RowState::default() },
            common: Common { data: place, ..Common::default() },
        }
    }
}

/// The From dropdown's provider, drawn inline.
fn google() -> Leading {
    Leading::Mark(
        rsx! { ProviderMark { provider: MarkProvider::Google, size: ControlSize::Small, style: MarkStyle::Letter } },
    )
}

pub const CASES: &[Case] = &[
    Case {
        name: "controls/button/leading-mark",
        make: || rsx! { Button { bezel: Bezel::Inline, label: "poh@acme.example", leading: google(), trailing: Icon::ChevronDown, shown: false, onclick: |_| {} } },
    },
    Case {
        name: "controls/button/leading-glyph",
        make: || rsx! { Button { size: ControlSize::Mini, label: "Pinned", leading: Leading::Glyph(Icon::Pin), onclick: |_| {} } },
    },
    Case {
        name: "lists/row/place-drop-accepts",
        make: || archive(DropState::Accepts),
    },
    Case {
        name: "controls/button/label-runs",
        make: || rsx! { Button { bezel: Bezel::Inline, label: quoted_head(), onclick: |_| {} } },
    },
    Case {
        name: "controls/button/label-runs-named",
        make: || rsx! { Button { common: Common { aria_label: Some("Show the quoted message".to_string()), ..Common::default() }, bezel: Bezel::Inline, label: quoted_head(), onclick: |_| {} } },
    },
];
