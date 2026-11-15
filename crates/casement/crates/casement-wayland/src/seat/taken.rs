//! Presses taken before a client saw them. A press the model swallowed (it only activated the
//! window) or a key a binding carried out never reached a client, so its release must not
//! reach one either: a client that sees a release without its press mis-tracks the button or
//! the key.

use casement_core::input::{ButtonCode, KeyCode};
use std::collections::BTreeSet;

/// The buttons and keys whose presses were taken and are still down.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Taken {
    buttons: BTreeSet<ButtonCode>,
    keys: BTreeSet<KeyCode>,
}

impl Taken {
    /// Nothing taken.
    pub fn none() -> Taken {
        Taken::default()
    }

    /// `button`'s press was taken.
    pub fn take_button(mut self, button: ButtonCode) -> Taken {
        self.buttons.insert(button);
        self
    }

    /// `key`'s press was taken.
    pub fn take_key(mut self, key: KeyCode) -> Taken {
        self.keys.insert(key);
        self
    }

    /// `button` was released: what became of its press. The press stops being remembered.
    pub fn release_button(mut self, button: ButtonCode) -> (Taken, bool) {
        let fate = fate_of(self.buttons.take(&button));
        (self, fate)
    }

    /// `key` was released: what became of its press. The press stops being remembered.
    pub fn release_key(mut self, key: KeyCode) -> (Taken, bool) {
        let fate = fate_of(self.keys.take(&key));
        (self, fate)
    }
}

/// Whether a release answers a press a client never saw (`true`: the press was taken, so the
/// release is taken too), given the press's entry if it was one taken.
fn fate_of<T>(taken: Option<T>) -> bool {
    taken.is_some()
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
