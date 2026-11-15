//! The key part of a chord.

use keyboard_types::NamedKey;

use crate::modifier::modifier_of_key;

/// The function keys a chord may name, F1 to F24, in order.
const FUNCTION_KEYS: [NamedKey; 24] = [
    NamedKey::F1,
    NamedKey::F2,
    NamedKey::F3,
    NamedKey::F4,
    NamedKey::F5,
    NamedKey::F6,
    NamedKey::F7,
    NamedKey::F8,
    NamedKey::F9,
    NamedKey::F10,
    NamedKey::F11,
    NamedKey::F12,
    NamedKey::F13,
    NamedKey::F14,
    NamedKey::F15,
    NamedKey::F16,
    NamedKey::F17,
    NamedKey::F18,
    NamedKey::F19,
    NamedKey::F20,
    NamedKey::F21,
    NamedKey::F22,
    NamedKey::F23,
    NamedKey::F24,
];

/// The key of a chord: keyboard-types' [`keyboard_types::Key`] narrowed to one character, so a
/// chord stays `Copy`.
///
/// A modifier key (`Named(NamedKey::Meta)`, ...) is that modifier pressed and released on its
/// own; `Chord::new` drops any other modifier from such a chord. The space bar is
/// `Char(' ')`, as keyboard-types writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// A character key, lower case (the key without Shift applied).
    Char(char),
    /// A named key.
    Named(NamedKey),
}

impl Key {
    /// The space bar.
    pub const SPACE: Key = Key::Char(' ');

    /// A character key; letters are lower-cased so `Z` and `z` are one key.
    pub fn character(c: char) -> Key {
        Key::Char(c.to_lowercase().next().unwrap_or(c))
    }

    /// Function key `n`, when `n` is 1 to 24.
    pub fn function(n: u8) -> Option<Key> {
        let index = usize::from(n).checked_sub(1)?;
        FUNCTION_KEYS.get(index).copied().map(Key::Named)
    }

    /// The key a toolkit reported as `key` (the logical key with Shift not applied): a named
    /// key, or a character key when its text is one character. `None` for longer text, which
    /// is a composed string and never a chord key.
    pub fn from_logical(key: &keyboard_types::Key) -> Option<Key> {
        match key {
            keyboard_types::Key::Named(named) => Some(Key::Named(*named)),
            keyboard_types::Key::Character(text) => {
                let mut chars = text.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => Some(Key::character(c)),
                    _ => None,
                }
            }
        }
    }

    /// The number of a function key, 1 to 24.
    pub(crate) fn function_number(self) -> Option<u8> {
        let Key::Named(named) = self else {
            return None;
        };
        let index = FUNCTION_KEYS.iter().position(|f| *f == named)?;
        u8::try_from(index + 1).ok()
    }

    /// Whether this key is a modifier pressed on its own.
    pub(crate) fn is_tap(self) -> bool {
        matches!(self, Key::Named(named) if modifier_of_key(named).is_some())
    }

    /// Whether this is a character a person types: a letter, digit, punctuation or symbol.
    pub(crate) fn is_printable(self) -> bool {
        matches!(self, Key::Char(c) if !c.is_whitespace() && !c.is_control())
    }

    /// Whether this key is not a shortcut key on its own inside a text field: characters,
    /// the space bar and the editing keys belong to the field until a modifier joins them.
    pub(crate) fn belongs_to_text_field(self) -> bool {
        matches!(
            self,
            Key::Char(_)
                | Key::Named(
                    NamedKey::Tab
                        | NamedKey::Enter
                        | NamedKey::Backspace
                        | NamedKey::Delete
                        | NamedKey::Insert
                        | NamedKey::Home
                        | NamedKey::End
                        | NamedKey::PageUp
                        | NamedKey::PageDown
                        | NamedKey::ArrowUp
                        | NamedKey::ArrowDown
                        | NamedKey::ArrowLeft
                        | NamedKey::ArrowRight
                )
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn characters_fold_case_and_function_keys_are_bounded() {
        assert_eq!(Key::character('Z'), Key::Char('z'));
        assert!(Key::Char('d').is_printable() && Key::Char('@').is_printable());
        assert!(!Key::SPACE.is_printable());
        assert!(!Key::Named(NamedKey::Tab).is_printable());
        assert_eq!(Key::function(0), None);
        assert_eq!(Key::function(1), Some(Key::Named(NamedKey::F1)));
        assert_eq!(Key::function(24), Some(Key::Named(NamedKey::F24)));
        assert_eq!(Key::function(25), None);
        assert_eq!(Key::Named(NamedKey::F7).function_number(), Some(7));
    }

    #[test]
    fn a_toolkit_key_becomes_a_chord_key() {
        use keyboard_types::Key as Typed;
        assert_eq!(
            Key::from_logical(&Typed::Character("Z".into())),
            Some(Key::Char('z'))
        );
        assert_eq!(
            Key::from_logical(&Typed::Character(" ".into())),
            Some(Key::SPACE)
        );
        assert_eq!(
            Key::from_logical(&Typed::Named(NamedKey::ArrowUp)),
            Some(Key::Named(NamedKey::ArrowUp))
        );
        assert_eq!(Key::from_logical(&Typed::Character("ab".into())), None);
    }

    #[test]
    fn text_fields_keep_typing_and_editing_keys() {
        assert!(Key::Char('a').belongs_to_text_field());
        assert!(Key::SPACE.belongs_to_text_field());
        assert!(Key::Named(NamedKey::Backspace).belongs_to_text_field());
        assert!(!Key::Named(NamedKey::Escape).belongs_to_text_field());
        assert!(!Key::Named(NamedKey::F5).belongs_to_text_field());
        assert!(!Key::Named(NamedKey::Meta).belongs_to_text_field());
        assert!(!Key::Named(NamedKey::AudioVolumeUp).belongs_to_text_field());
    }
}
