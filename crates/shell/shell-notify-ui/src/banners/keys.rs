//! The banner column's keymap: the keys a card takes once the column holds the keyboard, beyond
//! the ones the UI kit's card takes itself (Enter and Space press the card or the button that has the
//! focus; Tab moves between the card's close button, its buttons and the next card by the
//! document's own order). Up and Down move along the stack as it is drawn, newest on top;
//! Escape, Delete and Backspace dismiss.

use dioxus::prelude::{Key, NamedKey};

use super::deck_keys::{KeysIn, Neighbour};

/// What `key` does to the column; `None` leaves it to the document.
pub(super) fn banner_key(key: &Key) -> Option<KeysIn> {
    match key {
        // The newest card is the top one; older ones hang below it.
        Key::Named(NamedKey::ArrowUp) => Some(KeysIn::Moved(Neighbour::Newer)),
        Key::Named(NamedKey::ArrowDown) => Some(KeysIn::Moved(Neighbour::Older)),
        Key::Named(NamedKey::Escape)
        | Key::Named(NamedKey::Delete)
        | Key::Named(NamedKey::Backspace) => Some(KeysIn::Dismissed),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::{Key, NamedKey};

    use super::banner_key;
    use crate::banners::deck_keys::KeysIn::{Dismissed, Moved};
    use crate::banners::deck_keys::Neighbour::{Newer, Older};

    #[test]
    fn arrows_follow_the_stack_as_drawn_and_three_keys_dismiss() {
        let cases = [
            ("up", Key::Named(NamedKey::ArrowUp), Some(Moved(Newer))),
            ("down", Key::Named(NamedKey::ArrowDown), Some(Moved(Older))),
            ("Escape", Key::Named(NamedKey::Escape), Some(Dismissed)),
            ("Delete", Key::Named(NamedKey::Delete), Some(Dismissed)),
            (
                "Backspace",
                Key::Named(NamedKey::Backspace),
                Some(Dismissed),
            ),
            ("Enter is bardo's", Key::Named(NamedKey::Enter), None),
            ("Tab is the document's", Key::Named(NamedKey::Tab), None),
            ("a letter", Key::Character("x".into()), None),
        ];
        for (name, key, want) in cases {
            assert_eq!(banner_key(&key), want, "{name}");
        }
    }
}
