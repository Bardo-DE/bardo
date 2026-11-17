//! The mail-app states, as data: the name each renders under and how to make it.

use dioxus::prelude::*;
use ui_components::components::app::pin_tile::{PinFace, PinTile};
use ui_components::components::content::mark_face::MarkFace;
use ui_components::components::content::provider_mark::{MarkProvider, MarkStyle};
use ui_components::components::controls::button_model::Bezel;
use ui_components::prelude::*;
use ui_style::tokens::control_size::ControlSize;
use ui_style::tokens::hex::{Colour, Hex};

/// An account colour.
const SLATE: Colour = Colour::Solid(Hex([0x2f, 0x7f, 0x6e]));

/// A local-folders account: no provider, so the neutral folder mark.
fn local() -> PinFace {
    PinFace::Account {
        initial: 'L',
        colour: SLATE,
        provider: MarkProvider::Local,
        address: None,
    }
}

/// One state and its name.
pub struct Case {
    pub name: &'static str,
    pub make: fn() -> Element,
}

pub const CASES: &[Case] = &[
    Case {
        name: "lists/pin_tile/one-local",
        make: || rsx! { PinTile { face: local(), selection: true, unread: 2, onclick: |_| {} } },
    },
    Case {
        name: "lists/provider_mark/local-row",
        make: || rsx! { ProviderMark { provider: MarkProvider::Local, size: ControlSize::Mini, style: MarkStyle::Letter } },
    },
    Case {
        name: "lists/provider_mark/local-image-ignored",
        make: || rsx! { ProviderMark { provider: MarkProvider::Local, size: ControlSize::Small, style: MarkStyle::Image(ImageSource("data:image/png;base64,iVBORw0KGgo=".to_string())) } },
    },
    Case {
        name: "lists/provider_mark/face-one-letter",
        make: || rsx! { ProviderMark { provider: MarkProvider::Imap, size: ControlSize::Regular, face: Some(MarkFace::new("A", "#D97757")) } },
    },
    Case {
        name: "lists/provider_mark/face-two-letters-mini",
        make: || rsx! { ProviderMark { provider: MarkProvider::Imap, size: ControlSize::Mini, face: Some(MarkFace::new("OR", "#6467F2")) } },
    },
    Case {
        name: "lists/provider_mark/face-dark-ink",
        make: || rsx! { ProviderMark { provider: MarkProvider::Imap, size: ControlSize::Small, face: Some(MarkFace::new("Cx", "#FFE066")) } },
    },
    Case {
        name: "lists/provider_mark/face-bad-colour-neutral",
        make: || rsx! { ProviderMark { provider: MarkProvider::Google, size: ControlSize::Small, face: Some(MarkFace::new("Zz", "teal")) } },
    },
    Case {
        name: "controls/button/frame",
        make: || rsx! { Button { bezel: Bezel::Toolbar, label: "Settings", icon: Some(Icon::Settings), onclick: |_| {} } },
    },
    Case {
        name: "controls/button/frame-pressed",
        make: || rsx! { Button { bezel: Bezel::Toolbar, label: "Today", value: Some(Check::On), onclick: |_| {} } },
    },
    Case {
        name: "controls/button/quiet-caret",
        make: || rsx! { Button { bezel: Bezel::Inline, label: "poh@acme.example", trailing: Icon::ChevronDown, shown: false, onclick: |_| {} } },
    },
    Case {
        name: "controls/button/mini-trailing-glyph",
        make: || rsx! { Button { size: ControlSize::Mini, label: "Open", trailing: Icon::Link, onclick: |_| {} } },
    },
    Case {
        name: "controls/text_field/secret-value-unwritten",
        make: || rsx! { TextField { kind: FieldKind::Secure, label: "App password", value: "hunter2", placeholder: "App password", oninput: |_| {} } },
    },
    Case {
        name: "controls/text_field/bare",
        make: || rsx! { TextField { bezeled: false, label: "Name", value: "Work", placeholder: "Name this Space", oninput: |_| {} } },
    },
];
