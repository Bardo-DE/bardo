//! One shortcut: its name, who set it, its chord and, for one the person changed, Reset.

use super::field::ShortcutCell;
use crate::copy::shortcuts::{shortcut_note, standard_action_name};
use crate::key_row::row_id;
use dioxus::prelude::*;
use settings_app_model::{Origin, ShortcutRow, ShortcutsInput};
use ui_components::components::controls::button_model::Bezel;
use ui_components::components::fields::field_row::FieldRow;
use ui_components::prelude::*;
use ui_components::style::tokens::control_size::ControlSize;

/// A row of the list.
#[component]
pub(super) fn ShortcutLine(row: ShortcutRow, drive: EventHandler<ShortcutsInput>) -> Element {
    let action = row.action;
    let name = standard_action_name(action);
    rsx! {
        FieldRow {
            label: TextLine::from(name.clone()),
            help: Some(TextLine::from(shortcut_note(&row))),
            common: Common { id: Some(row_id("shortcut", &name)), ..Common::default() },
            ShortcutCell { row: row.clone(), drive }
            if row.origin == Origin::You {
                Button {
                    common: Common {
                        aria_label: Some(crate::copy::shortcuts::reset(&name)),
                        ..Common::default()
                    },
                    label: crate::copy::shortcuts::RESET,
                    bezel: Bezel::Inline,
                    size: ControlSize::Small,
                    onclick: move |_| drive.call(ShortcutsInput::Reset(action)),
                }
            }
        }
    }
}
