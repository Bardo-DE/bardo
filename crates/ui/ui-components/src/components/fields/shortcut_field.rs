//! ShortcutField: the control that records a keyboard shortcut (System Settings' shortcut
//! recorder). It shows the chord it was given; a click, or Return or Space with the keyboard on
//! it, makes it listen ("Type shortcut"); the next combination is offered to the caller, Escape
//! gives up, Backspace or Delete clears. While it listens it keeps every key it hears from the
//! window's actions.
//!
//! `single_keys` chooses what counts: combinations, or single keys (a bare key, or Shift with a
//! character; Delete is then a key). `more` lists the action's further keys, drawn as caps beside
//! `value`.
//!
//! Markup: `span.ds-shortcut-field[data-recording][data-clash]` holding
//! `button.ds-shortcut-field-well` (with `more`, a `span.ds-shortcut-field-caps` of a `kbd.ds-key-equivalent-key` cap per key, KeyEquivalent's cap style) and, when another action uses the chord, a
//! `span.ds-shortcut-field-clash`.

use crate::components::controls::press::{ActivationKeys, activates, disabled};
use crate::components::fields::shortcut_field_model::{
    Heard, ShortcutClash, ShortcutRecorded, hear,
};
use crate::focus::soon::focus_soon;
use crate::keys::use_platform;
use crate::root::common::Common;
use keymap::Chord;
use dioxus::prelude::*;
use ui_core::command::chord_text;
use ui_core::time::{FRAME_SLACK, clock::sleep};
use ui_core::vocab::Availability;
use ui_core::word::Word;

/// What the well reads while it is not listening.
const EMPTY: &str = "Add shortcut";
/// What the well reads while it listens.
const LISTENING: &str = "Type shortcut";

/// A shortcut recorder. `value` is the shortcut shown (the caller holds it and stores what
/// `onrecord` reports); `label` names it to assistive technology. `clash` names the action that
/// already uses the shortcut, shown under the field.
#[component]
pub fn ShortcutField(
    label: String,
    #[props(default)] value: Option<Chord>,
    onrecord: EventHandler<ShortcutRecorded>,
    /// What the field accepts: single keys when set, combinations (the default) otherwise.
    #[props(default)]
    single_keys: bool,
    /// Further keys the action also has, drawn as key caps after `value`. The caller manages
    /// them; recording replaces only `value`.
    #[props(default)]
    more: Vec<Chord>,
    #[props(default)] clash: ShortcutClash,
    #[props(default)] availability: Availability,
    #[props(default)] common: Common,
) -> Element {
    let platform = use_platform();
    // Whether the field is listening for a combination, or showing its shortcut.
    let mut listening = use_signal(|| false);
    // The well itself: a click hands it the keyboard, so the next combination reaches it.
    let mut well = use_signal(|| None::<std::rc::Rc<MountedData>>);
    // Whether the well has the keyboard, so a blur that the click's own focus undoes is not a
    // cancel.
    let mut held = use_signal(|| false);
    let live = availability == Availability::Enabled;
    let mut finish = move |recorded: ShortcutRecorded| {
        listening.set(false);
        onrecord.call(recorded);
    };
    let text = match (listening(), value) {
        (true, _) => LISTENING.to_owned(),
        (false, Some(chord)) => chord_text(platform, &chord),
        (false, None) => EMPTY.to_owned(),
    };
    let extras: Vec<String> = more
        .iter()
        .map(|chord| chord_text(platform, chord))
        .collect();
    let class = common.class("ds-shortcut-field");
    let data = common.data_attributes();
    let name = common.aria_label.clone().unwrap_or(label);
    let used_by = match &clash {
        ShortcutClash::None => None,
        ShortcutClash::With(action) => Some(format!("Used by {action}")),
    };
    rsx! {
        span {
            class,
            "data-recording": if listening() { "listening" } else { "idle" },
            "data-clash": used_by.is_some().then_some("with"),
            "data-availability": availability.slug(),
            ..data,
            button {
                r#type: "button",
                class: "ds-shortcut-field-well",
                id: common.id.clone(),
                "aria-label": "{name}",
                "aria-pressed": listening().then_some("true"),
                disabled: disabled(availability),
                onclick: move |_| {
                    if live {
                        listening.set(true);
                        if let Some(element) = well() {
                            focus_soon(element);
                        }
                    }
                },
                onfocus: move |_| held.set(true),
                onblur: move |_| {
                    held.set(false);
                    // Blitz can blur the well around the click that starts listening (it moves
                    // focus off a focused button whose child was hit) and the click focuses it
                    // again; only a blur that still stands a frame later gives up.
                    spawn(async move {
                        sleep(FRAME_SLACK).await;
                        if !held() && listening() {
                            finish(ShortcutRecorded::Cancelled);
                        }
                    });
                },
                onkeydown: move |event: KeyboardEvent| match listening() {
                    false => {
                        if live && activates(&event, ActivationKeys::ReturnAndSpace) {
                            event.prevent_default();
                            event.stop_propagation();
                            listening.set(true);
                        }
                    }
                    true => {
                        let heard = hear(platform, single_keys, &event.key(), event.modifiers());
                        if heard == Heard::Leave {
                            return;
                        }
                        event.prevent_default();
                        event.stop_propagation();
                        match heard {
                            Heard::Offer(chord) => finish(ShortcutRecorded::Chord(chord)),
                            Heard::Clear => finish(ShortcutRecorded::Cleared),
                            Heard::Cancel => finish(ShortcutRecorded::Cancelled),
                            Heard::Wait | Heard::Leave => {}
                        }
                    }
                },
                onmounted: move |event| {
                    well.set(Some(event.data()));
                    common.mounted(event);
                },
                if more.is_empty() {
                    "{text}"
                } else {
                    span { class: "ds-shortcut-field-caps ds-key-equivalent", "data-style": "cap",
                        kbd { class: "ds-key-equivalent-key", "{text}" }
                        for extra in &extras {
                            kbd { class: "ds-key-equivalent-key", "{extra}" }
                        }
                    }
                }
            }
            if let Some(used_by) = &used_by {
                span { class: "ds-shortcut-field-clash", role: "status", "{used_by}" }
            }
        }
    }
}
