//! Whether a pressed control takes the keyboard: the scope `FocusOnPressScope` that sets it for
//! every control inside.
//!
//! On macOS a control in a formatting bar (an `NSSegmentedControl` that refuses first
//! responder, the Pages and `TextEdit` format bars) acts on a press and leaves the keyboard, the
//! caret and the selection in the text view. A browser button takes the keyboard; Blitz clears it
//! on any click nothing took. [`FocusOnPressScope`] puts `refuses: true` around a bar,
//! and each Bardo control inside (`Button`, `IconButton`, `SegmentedControl`) then prevents its
//! click's default, so Blitz leaves the focus where it was, and hands the click to no host: the
//! text body keeps the keyboard, and a later command-I still reaches it. The scope is a context,
//! not a prop on each control, so one wrapper covers every control in the bar.

use dioxus::prelude::*;

/// Whether the controls inside refuse the keyboard (the scope's context).
#[derive(Clone, Copy)]
struct Refuses(bool);

/// Put `refuses` around every control inside: a control that refuses acts on the press and leaves
/// the keyboard where it was; one that does not takes it, as a button does (the default).
#[component]
pub fn FocusOnPressScope(refuses: bool, children: Element) -> Element {
    use_context_provider(|| Refuses(refuses));
    rsx! { {children} }
}

/// Whether the nearest [`FocusOnPressScope`] refuses the keyboard; outside one, no.
fn refuses_focus() -> bool {
    try_consume_context::<Refuses>().is_some_and(|Refuses(refuses)| refuses)
}

/// A control's click: under a scope that refuses, keep the renderer from moving the keyboard.
/// Returns whether the control then refuses the keyboard.
pub(crate) fn on_click(event: &MouseEvent) -> bool {
    let refuses = refuses_focus();
    if refuses {
        event.prevent_default();
    }
    refuses
}
