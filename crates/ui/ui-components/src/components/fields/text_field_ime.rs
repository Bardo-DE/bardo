//! Whether an input method is composing in a `TextField`. Its preedit draws in the field before
//! it is the field's value, so the value alone would leave the placeholder over the pinyin being
//! typed. dioxus-native-dom drops IME events, so no `oncomposition*` handler ever runs; the field
//! registers with the window's IME as an edit surface does (`host::ime`) and hears the preedit
//! beside the renderer, which still draws it and commits it into the input.

use crate::host::document::DocumentHost;
use crate::host::ime::{ImeEvent, ImeListener};
use crate::host::measure::BUSY_ATTEMPTS;
use crate::host::probe::Probe;
use base_machine::time::{FRAME_SLACK, clock::sleep};
use dioxus::prelude::*;
use std::rc::Rc;
use ui_style::task::{spawn_in, try_set_if_changed};

/// Whether an input method has preedit text in the field, after `event` when it was `composing`
/// before: composing while there is preedit text, not once it is cleared, committed or the IME
/// detaches. The IME attaching changes nothing.
fn composing_after(composing: bool, event: &ImeEvent) -> bool {
    match event {
        ImeEvent::Preedit { text, .. } => !text.is_empty(),
        ImeEvent::Commit(_) | ImeEvent::Disabled => false,
        ImeEvent::Enabled => composing,
    }
}

/// The field's composing state and its registration with the window's IME.
#[derive(Clone, Copy)]
pub(crate) struct FieldIme {
    state: Signal<bool>,
    heard: EventHandler<ImeEvent>,
    listener: CopyValue<Option<ImeListener>>,
    /// The document's host, when one was provided: without one nothing is heard.
    host: CopyValue<Option<Rc<dyn DocumentHost>>>,
    owner: ScopeId,
}

impl FieldIme {
    /// The hook: one per field, kept across renders; its registration ends with the field.
    pub(crate) fn use_new() -> Self {
        let state = use_signal(|| false);
        let ime = FieldIme {
            state,
            heard: use_hook(|| {
                EventHandler::new(move |event: ImeEvent| {
                    let next = composing_after(*state.peek(), &event);
                    let _ = try_set_if_changed(state, next);
                })
            }),
            listener: use_hook(|| CopyValue::new(None)),
            host: use_hook(|| CopyValue::new(try_consume_context::<Rc<dyn DocumentHost>>())),
            owner: use_hook(dioxus::core::current_scope_id),
        };
        use_drop(move || ime.forget());
        ime
    }

    /// Whether an input method is composing now.
    pub(crate) fn composing(&self) -> bool {
        (self.state)()
    }

    /// The caret left the field: whatever the IME was composing there is over.
    pub(crate) fn left(&self) {
        let _ = try_set_if_changed(self.state, false);
    }

    /// Register the field's element for IME events once the document is free.
    pub(crate) fn mounted(&self, event: &MountedEvent) {
        let Some(host) = self.host.peek().clone() else {
            return;
        };
        let element = event.data();
        let ime = *self;
        spawn_in(self.owner, async move {
            let Some(edit) = host.edit() else {
                return;
            };
            for _ in 0..BUSY_ATTEMPTS {
                match edit.ime().listen(&element, ime.heard) {
                    Probe::Busy => sleep(FRAME_SLACK).await,
                    Probe::Found(listener) => {
                        let mut slot = ime.listener;
                        if let Some(stale) = slot.replace(Some(listener)) {
                            edit.ime().forget(stale);
                        }
                        return;
                    }
                    Probe::Unknown => return,
                }
            }
        });
    }

    fn forget(&self) {
        let mut slot = self.listener;
        if let (Some(host), Some(listener)) = (self.host.peek().clone(), slot.take())
            && let Some(edit) = host.edit()
        {
            edit.ime().forget(listener);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::composing_after;
    use crate::host::ime::ImeEvent;

    fn preedit(text: &str) -> ImeEvent {
        ImeEvent::Preedit {
            text: text.to_owned(),
            cursor: None,
        }
    }

    #[test]
    fn composing_lasts_from_the_first_preedit_to_the_commit() {
        let cases: &[(bool, ImeEvent, bool)] = &[
            (false, ImeEvent::Enabled, false),
            (false, preedit("n"), true),
            (true, preedit("ni"), true),
            (true, ImeEvent::Enabled, true),
            (true, preedit(""), false),
            (true, ImeEvent::Commit("你".to_owned()), false),
            (true, ImeEvent::Disabled, false),
        ];
        for (before, event, after) in cases {
            assert_eq!(
                composing_after(*before, event),
                *after,
                "{before:?} then {event:?}"
            );
        }
    }
}
