//! RowConfirm: a question a row asks in its own line, in place of its words and accessory, with
//! a Cancel and a confirming button (design/30 section 2.6). It is the row-scoped sibling of an
//! `Alert`: where the alert interrupts the window, this one sits where the person pressed, so
//! removing one row of a long list asks about that row only. Cancel takes the keyboard as it
//! appears and Escape answers it; the row is not picked while it asks.

use crate::components::controls::button::Button;
use crate::components::controls::button_model::{Answers, Bezel};
use crate::focus::click::kept_click;
use crate::focus::soon::focus_soon;
use crate::root::common::Common;
use dioxus::prelude::*;
use ds_style::tokens::control_size::ControlSize;

/// The question a row asks.
#[derive(Debug, Clone, PartialEq)]
pub struct RowConfirm {
    /// The question, in the title's place: "Delete the folder and its 12 messages?"
    pub question: String,
    /// The confirming button's words: a verb ("Delete"), never "OK".
    pub confirm: String,
    /// Whether confirming deletes or discards.
    pub destructive: bool,
    /// The confirming button was pressed.
    pub on_confirm: EventHandler<()>,
    /// Cancel was pressed or Escape was pressed.
    pub on_cancel: EventHandler<()>,
}

/// The question in the words' place.
pub(crate) fn question(confirm: &RowConfirm) -> Element {
    rsx! {
        span { class: "ds-row-words",
            b { class: "ds-row-title ds-truncate", "{confirm.question}" }
        }
    }
}

/// The two buttons, at the row's end: Cancel, then the confirming one.
pub(crate) fn buttons(confirm: &RowConfirm) -> Element {
    let RowConfirm {
        confirm: verb,
        destructive,
        on_confirm,
        on_cancel,
        ..
    } = confirm.clone();
    rsx! {
        span {
            class: "ds-row-confirm",
            onkeydown: move |event| {
                if event.key() == Key::Escape {
                    event.stop_propagation();
                    on_cancel.call(());
                }
            },
            // Nothing pressed here reaches the row.
            onmousedown: move |event| event.stop_propagation(),
            onclick: move |event| {
                event.stop_propagation();
                kept_click(&event);
            },
            Button {
                label: "Cancel",
                bezel: Bezel::Push,
                size: ControlSize::Small,
                answers: Answers::Escape,
                stop_propagation: true,
                onclick: move |_| on_cancel.call(()),
                common: Common {
                    mounted: Some(EventHandler::new(|event: MountedEvent| focus_soon(event.data()))),
                    ..Common::default()
                },
            }
            Button {
                label: verb,
                bezel: Bezel::Push,
                size: ControlSize::Small,
                destructive,
                stop_propagation: true,
                onclick: move |_| on_confirm.call(()),
            }
        }
    }
}
