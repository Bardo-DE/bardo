//! Presses taken before a client saw them. A press the model swallowed (it only activated the
//! window) or a key a binding carried out never reached a client, so its release must not
//! reach one either: a client that sees a release without its press mis-tracks the button or
//! the key.

use compositor_core::input::{ButtonCode, KeyCode};
use std::collections::BTreeSet;

/// The buttons and keys whose presses were taken and are still down.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Taken {
    buttons: BTreeSet<ButtonCode>,
    keys: BTreeSet<KeyCode>,
}

impl Taken {
    /// Nothing taken.
    #[must_use]
    pub fn none() -> Taken {
        Taken::default()
    }

    /// `button`'s press was taken.
    #[must_use]
    pub fn take_button(mut self, button: ButtonCode) -> Taken {
        self.buttons.insert(button);
        self
    }

    /// `key`'s press was taken.
    #[must_use]
    pub fn take_key(mut self, key: KeyCode) -> Taken {
        self.keys.insert(key);
        self
    }

    /// `button` was released: what became of its press. The press stops being remembered.
    #[must_use]
    pub fn release_button(mut self, button: ButtonCode) -> (Taken, bool) {
        // A release answers a press a client never saw when its press was taken.
        let fate = self.buttons.take(&button).is_some();
        (self, fate)
    }

    /// `key` was released: what became of its press. The press stops being remembered.
    #[must_use]
    pub fn release_key(mut self, key: KeyCode) -> (Taken, bool) {
        let fate = self.keys.take(&key).is_some();
        (self, fate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_taken_press_takes_its_release_once() {
        let button = ButtonCode(0x110);
        let taken = Taken::none().take_button(button);
        let (taken, first) = taken.release_button(button);
        assert!(first);
        let (_, second) = taken.release_button(button);
        assert!(!second, "the next release is a client's own");
    }

    #[test]
    fn buttons_and_keys_are_remembered_apart() {
        let taken = Taken::none().take_key(KeyCode(0x110));
        let (taken, fate) = taken.release_button(ButtonCode(0x110));
        assert!(!fate);
        assert!(taken.release_key(KeyCode(0x110)).1);
    }
}
