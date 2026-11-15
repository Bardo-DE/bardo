//! A key handler that speaks actions: the one line an app puts on a surface.

use super::handle::Keys;
use chordkit::{Action, Context};
use dioxus::prelude::KeyboardEvent;

/// An `onkeydown` handler that resolves each key in `context` and hands the action to `handle`.
/// `handle` answers whether the app took the action: only a taken action consumes the key event;
/// one the app ignores goes on to the focused widget as usual.
///
/// ```ignore
/// let keys = use_keys();
/// rsx! { div { onkeydown: on_action(keys, Context::Normal, move |action| match action {
///     Action::Standard(StandardAction::Save) => { save(); true }
///     _ => false,
/// }) } }
/// ```
pub fn on_action(
    keys: Keys,
    context: Context,
    mut handle: impl FnMut(Action) -> bool + 'static,
) -> impl FnMut(KeyboardEvent) + 'static {
    move |event: KeyboardEvent| {
        let Some(action) = keys.action_of(&event, context) else {
            return;
        };
        if handle(action) == true {
            event.prevent_default();
            event.stop_propagation();
        }
    }
}
