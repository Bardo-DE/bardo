//! The mail-app states, as data: the name each renders under and how to make it.

use dioxus::prelude::*;
use ui_components::components::app::pin_tile::{PinFace, PinTile};
use ui_components::components::app::send_mood::SendMood;
use ui_components::components::app::send_pill::{PillAction, SendPill};
use ui_components::components::content::avatar::{
    AvatarFace, AvatarShape, AvatarSize, AvatarTone, PersonHue,
};
use ui_components::components::content::provider_mark::{MarkProvider, MarkStyle};
use ui_components::components::controls::button_model::{Bezel, ImagePosition};
use ui_components::prelude::*;
use ui_style::tokens::control_size::ControlSize;
use ui_style::tokens::hex::{Colour, Hex};

/// A scheduled draft's favicon.
const CLOCKED: AvatarFace = AvatarFace {
    initial: 'Q',
    size: AvatarSize::Size18,
    tone: AvatarTone::Person(PersonHue(212)),
    shape: AvatarShape::Round,
};

/// A scheduled Today row: its time and cancel, and no close.
fn scheduled() -> Element {
    rsx! {
        Row {
            title: "Q3 notes",
            leading: RowLeading::Avatar(CLOCKED),
            accessory: Accessory::Slot(rsx! {
                span { "Mon 9:00" }
                Button { bezel: Bezel::Toolbar, image: ImagePosition::Only, icon: Icon::X, label: "Cancel sending Q3 notes", stop_propagation: true, onclick: |_| {} }
            }),
        }
    }
}

/// An account colour.
const VIOLET: Colour = Colour::Solid(Hex([0x5b, 0x4f, 0xc4]));

/// Poh's account on Google.
fn poh() -> PinFace {
    PinFace::Account {
        initial: 'P',
        colour: VIOLET,
        provider: MarkProvider::Google,
        address: Some("poh@acme.example".to_string()),
    }
}

/// One state and its name.
pub struct Case {
    pub name: &'static str,
    pub make: fn() -> Element,
}

pub const CASES: &[Case] = &[
    // Button: a hint, a name for assistive technology, and the open state of what it opens.
    Case {
        name: "controls/button/mini-titled-open",
        make: || rsx! { Button { common: Common { aria_label: Some("Add account".to_string()), ..Common::default() }, size: ControlSize::Mini, label: "+", title: "Add Account", shown: true, onclick: |_| {} } },
    },
    Case {
        name: "controls/button/quiet-closed",
        make: || rsx! { Button { bezel: Bezel::Inline, label: "More", shown: false, onclick: |_| {} } },
    },
    // TextField: a secure field, empty (the placeholder) and filled (the dots).
    Case {
        name: "controls/text_field/password-empty",
        make: || rsx! { TextField { kind: FieldKind::Secure, label: "Password", value: "", placeholder: "App password", oninput: |_| {} } },
    },
    Case {
        name: "controls/text_field/password-filled",
        make: || rsx! { TextField { kind: FieldKind::Secure, label: "Password", value: "hunter2", oninput: |_| {} } },
    },
    // PinTile: the favicon mark, and the Add account tile, with and without a hint.
    Case {
        name: "lists/pin_tile/one-image-mark",
        make: || rsx! { PinTile { face: poh(), selection: true, unread: 2, mark: MarkStyle::Image(ImageSource("data:image/png;base64,iVBORw0KGgo=".to_string())), onclick: |_| {} } },
    },
    Case {
        name: "lists/pin_tile/add",
        make: || rsx! { PinTile { face: PinFace::Add { label: "Add account".to_string(), hint: Some("Add Account".to_string()) }, onclick: |_| {} } },
    },
    Case {
        name: "lists/pin_tile/add-named",
        make: || rsx! { PinTile { face: PinFace::Add { label: "Add an account to Work".to_string(), hint: None }, onclick: |_| {} } },
    },
    // SendPill: Cancel for a held send, and a fatal refusal on two lines.
    Case {
        name: "overlays/send_pill/cancel",
        make: || rsx! { SendPill { text: "Scheduled for 9:00", progress: Fraction(0), operation: Operation::Running(PendingToken::start()), action: PillAction::Cancel, onundo: |()| {} } },
    },
    Case {
        name: "overlays/send_pill/fatal-refused",
        make: || rsx! { SendPill { text: "Not sent", progress: Fraction(700), operation: Operation::Running(PendingToken::start()), mood: SendMood::Fatal, refusal: "No recipients", onundo: |()| {} } },
    },
    // Row: a scheduled Today row with its time and cancel.
    Case {
        name: "lists/row/today-scheduled",
        make: scheduled,
    },
    // Avatar: an account's colour muted (chroma .55, hue and lightness kept),
    // and a person's; a muted ink avatar is greyscale already and keeps its colours.
    Case {
        name: "controls/avatar/account-28-muted",
        make: || rsx! { Avatar { initial: 'P', size: AvatarSize::Size28, tone: AvatarTone::Account(VIOLET), muting: true } },
    },
    Case {
        name: "controls/avatar/person-18-muted",
        make: || rsx! { Avatar { initial: 'D', size: AvatarSize::Size18, tone: AvatarTone::Person(PersonHue(212)), muting: true } },
    },
    Case {
        name: "controls/avatar/ink-28-muted",
        make: || rsx! { Avatar { initial: 'A', size: AvatarSize::Size28, tone: AvatarTone::Ink, muting: true } },
    },
];
