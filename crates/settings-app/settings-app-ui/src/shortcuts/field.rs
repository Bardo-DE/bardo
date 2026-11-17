//! How a chord is shown and changed in a row: Bardo's recorder. The one file that names it.

use crate::copy::shortcuts::{action_name, standard_action_name};
use dioxus::prelude::*;
use settings_app_model::{ShortcutRow, ShortcutsInput};
use ui_components::prelude::*;

/// The recorder of `row`: a chord recorded is set, Delete clears, Escape changes nothing. While
/// the person is asked whether to take a chord over, the recorder says who has it.
#[component]
pub(super) fn ShortcutCell(row: ShortcutRow, drive: EventHandler<ShortcutsInput>) -> Element {
    let action = row.action;
    let clash = row.clash.as_ref().map_or(ShortcutClash::None, |holder| {
        ShortcutClash::With(action_name(holder))
    });
    rsx! {
        ShortcutField {
            label: standard_action_name(action),
            value: row.chord,
            clash,
            onrecord: move |recorded: ShortcutRecorded| match recorded {
                ShortcutRecorded::Chord(chord) => drive.call(ShortcutsInput::Set(action, chord)),
                ShortcutRecorded::Cleared => drive.call(ShortcutsInput::Clear(action)),
                ShortcutRecorded::Cancelled => {}
            },
        }
    }
}
