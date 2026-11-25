//! The scroll key a winit key press is, if it is one.

use blitz_embed::scroll::keys::{ScrollKey, scroll_key};
use dioxus_native::winit::keyboard::{Key as WinitKey, NamedKey as WinitNamedKey};
use keyboard_types::{Key, Modifiers, NamedKey};

/// The scroll key `key` with `held` modifiers is: arrows, Page Up and Down, Home and End, Space.
pub(super) fn scroll_key_of(key: &WinitKey, held: Modifiers) -> Option<ScrollKey> {
    let named = match key {
        WinitKey::Named(WinitNamedKey::ArrowUp) => Key::Named(NamedKey::ArrowUp),
        WinitKey::Named(WinitNamedKey::ArrowDown) => Key::Named(NamedKey::ArrowDown),
        WinitKey::Named(WinitNamedKey::ArrowLeft) => Key::Named(NamedKey::ArrowLeft),
        WinitKey::Named(WinitNamedKey::ArrowRight) => Key::Named(NamedKey::ArrowRight),
        WinitKey::Named(WinitNamedKey::PageUp) => Key::Named(NamedKey::PageUp),
        WinitKey::Named(WinitNamedKey::PageDown) => Key::Named(NamedKey::PageDown),
        WinitKey::Named(WinitNamedKey::Home) => Key::Named(NamedKey::Home),
        WinitKey::Named(WinitNamedKey::End) => Key::Named(NamedKey::End),
        WinitKey::Character(text) if text.as_str() == " " => Key::Character(" ".into()),
        _ => return None,
    };
    scroll_key(&named, held)
}

#[cfg(test)]
mod tests {
    use super::*;
    use blitz_embed::scroll::geom::{Dir, ScrollAxis};

    #[test]
    fn the_keys_that_scroll() {
        // name, key, modifiers, the scroll key
        let cases = [
            (
                "down",
                WinitKey::Named(WinitNamedKey::ArrowDown),
                Modifiers::empty(),
                Some(ScrollKey::Line(ScrollAxis::Y, Dir::Pos)),
            ),
            (
                "page down",
                WinitKey::Named(WinitNamedKey::PageDown),
                Modifiers::empty(),
                Some(ScrollKey::Page(Dir::Pos)),
            ),
            (
                "space",
                WinitKey::Character(" ".into()),
                Modifiers::empty(),
                Some(ScrollKey::Page(Dir::Pos)),
            ),
            (
                "shift space",
                WinitKey::Character(" ".into()),
                Modifiers::SHIFT,
                Some(ScrollKey::Page(Dir::Neg)),
            ),
            (
                "end",
                WinitKey::Named(WinitNamedKey::End),
                Modifiers::empty(),
                Some(ScrollKey::Edge(Dir::Pos)),
            ),
            (
                "alt down is a shortcut",
                WinitKey::Named(WinitNamedKey::ArrowDown),
                Modifiers::ALT,
                None,
            ),
            (
                "a letter",
                WinitKey::Character("j".into()),
                Modifiers::empty(),
                None,
            ),
            (
                "enter",
                WinitKey::Named(WinitNamedKey::Enter),
                Modifiers::empty(),
                None,
            ),
        ];
        for (name, key, held, want) in cases {
            assert_eq!(scroll_key_of(&key, held), want, "{name}");
        }
    }
}
