//! What the shortcut field decides from a key while it records: the pure step, apart from the
//! view that carries it out.

use chordkit::keyboard_types::Modifiers as ChordModifiers;
use chordkit::{Chord, Key as ChordKey, Platform};
use dioxus::prelude::{Key, Modifiers};
use ds_core::command::{chord_of, modifiers_of};
use ds_core::word::Word;

/// What one recording ended with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutRecorded {
    /// The person typed this combination.
    Chord(Chord),
    /// The person cleared the shortcut (Backspace or Delete).
    Cleared,
    /// The person gave up: Escape, or the keyboard went elsewhere.
    Cancelled,
}

/// Another action that already uses the combination in the field.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ShortcutClash {
    /// Nothing else uses it.
    #[default]
    None,
    /// The action with this name does; the field says so under itself.
    With(String),
}

/// What a key means to a field that is listening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Heard {
    /// A combination to offer.
    Offer(Chord),
    /// Clear the shortcut.
    Clear,
    /// Stop listening.
    Cancel,
    /// Not a shortcut yet (a lone modifier, a plain letter): the field keeps the key and goes on
    /// listening.
    Wait,
    /// Tab moving the keyboard on: not the field's.
    Leave,
}

/// What `key` with `modifiers` held means to a field listening on `platform`, for single keys
/// when `single_keys` is set and for combinations otherwise. Escape, Backspace, Delete and Tab
/// are meant bare; with a modifier they are combinations like any other. For single keys Delete
/// is offered as a key.
pub(crate) fn hear(
    platform: Platform,
    single_keys: bool,
    key: &Key,
    modifiers: Modifiers,
) -> Heard {
    let bare = modifiers_of(modifiers).is_empty();
    match (key, bare, single_keys) {
        (Key::Escape, true, _) => Heard::Cancel,
        (Key::Backspace, true, _) => Heard::Clear,
        (Key::Delete, true, false) => Heard::Clear,
        (Key::Tab, true, _) => Heard::Leave,
        _ => match chord_of(platform, key, modifiers)
            .filter(|chord| is_shortcut(single_keys, chord))
        {
            Some(chord) => Heard::Offer(chord),
            None => Heard::Wait,
        },
    }
}

/// Whether `chord` is something to bind. Combinations: it holds a modifier (Shift
/// alone with a character is typing), or it is a function key. Single keys: it is bare, or Shift
/// with a character.
fn is_shortcut(single_keys: bool, chord: &Chord) -> bool {
    let modifiers = chord.modifiers();
    if single_keys {
        let shifted =
            modifiers == ChordModifiers::SHIFT && matches!(chord.key(), ChordKey::Char(_));
        return modifiers.is_empty() || shifted;
    }
    let typing = modifiers == ChordModifiers::SHIFT && matches!(chord.key(), ChordKey::Char(_));
    let function = (1..=24).any(|n| ChordKey::function(n) == Some(chord.key()));
    function || (!modifiers.is_empty() && !typing)
}

#[cfg(test)]
mod tests {
    use super::{Heard, hear};
    use chordkit::keyboard_types::{Modifiers as ChordModifiers, NamedKey};
    use chordkit::{Chord, Desktop, Key as ChordKey, Platform};
    use dioxus::prelude::{Key, Modifiers};

    fn ours() -> Platform {
        Platform::Linux {
            desktop: Desktop::Ours,
        }
    }

    fn chord(modifiers: ChordModifiers, key: ChordKey) -> Heard {
        Heard::Offer(Chord::new(modifiers, key))
    }

    #[test]
    fn a_key_is_a_combination_a_command_or_nothing_yet() {
        let letter = |c: &str| Key::Character(c.to_owned());
        let cases: Vec<(&str, Key, Modifiers, Heard)> = vec![
            (
                "escape cancels",
                Key::Escape,
                Modifiers::empty(),
                Heard::Cancel,
            ),
            (
                "backspace clears",
                Key::Backspace,
                Modifiers::empty(),
                Heard::Clear,
            ),
            (
                "delete clears",
                Key::Delete,
                Modifiers::empty(),
                Heard::Clear,
            ),
            ("tab leaves", Key::Tab, Modifiers::empty(), Heard::Leave),
            (
                "caps lock does not make escape a combination",
                Key::Escape,
                Modifiers::CAPS_LOCK,
                Heard::Cancel,
            ),
            (
                "a bare letter waits",
                letter("k"),
                Modifiers::empty(),
                Heard::Wait,
            ),
            (
                "shift and a letter is typing",
                letter("K"),
                Modifiers::SHIFT,
                Heard::Wait,
            ),
            (
                "a lone modifier waits",
                Key::Shift,
                Modifiers::SHIFT,
                Heard::Wait,
            ),
            (
                "a bare arrow waits",
                Key::ArrowUp,
                Modifiers::empty(),
                Heard::Wait,
            ),
            (
                "command and a letter",
                letter("k"),
                Modifiers::SUPER,
                chord(ChordModifiers::META, ChordKey::Char('k')),
            ),
            (
                "shift command and a letter",
                letter("K"),
                Modifiers::SUPER | Modifiers::SHIFT,
                chord(
                    ChordModifiers::META | ChordModifiers::SHIFT,
                    ChordKey::Char('k'),
                ),
            ),
            (
                "control and an arrow",
                Key::ArrowLeft,
                Modifiers::CONTROL,
                chord(
                    ChordModifiers::CONTROL,
                    ChordKey::Named(NamedKey::ArrowLeft),
                ),
            ),
            (
                "a function key needs no modifier",
                Key::F5,
                Modifiers::empty(),
                chord(ChordModifiers::empty(), ChordKey::Named(NamedKey::F5)),
            ),
            (
                "option escape is a combination",
                Key::Escape,
                Modifiers::ALT,
                chord(ChordModifiers::ALT, ChordKey::Named(NamedKey::Escape)),
            ),
        ];
        for (name, key, modifiers, want) in cases {
            assert_eq!(hear(ours(), false, &key, modifiers), want, "{name}");
        }
    }

    #[test]
    fn single_keys_take_a_bare_key_or_shift_and_a_character() {
        let letter = |c: &str| Key::Character(c.to_owned());
        let cases: Vec<(&str, Key, Modifiers, Heard)> = vec![
            (
                "a bare letter is offered",
                letter("j"),
                Modifiers::empty(),
                chord(ChordModifiers::empty(), ChordKey::Char('j')),
            ),
            (
                "shift and a letter is offered",
                letter("J"),
                Modifiers::SHIFT,
                chord(ChordModifiers::SHIFT, ChordKey::Char('j')),
            ),
            (
                "delete is a key",
                Key::Delete,
                Modifiers::empty(),
                chord(ChordModifiers::empty(), ChordKey::Named(NamedKey::Delete)),
            ),
            (
                "backspace clears",
                Key::Backspace,
                Modifiers::empty(),
                Heard::Clear,
            ),
            (
                "escape cancels",
                Key::Escape,
                Modifiers::empty(),
                Heard::Cancel,
            ),
            ("tab leaves", Key::Tab, Modifiers::empty(), Heard::Leave),
            (
                "a bare arrow is offered",
                Key::ArrowUp,
                Modifiers::empty(),
                chord(ChordModifiers::empty(), ChordKey::Named(NamedKey::ArrowUp)),
            ),
            (
                "control and a letter waits",
                letter("j"),
                Modifiers::CONTROL,
                Heard::Wait,
            ),
            (
                "a lone modifier waits",
                Key::Shift,
                Modifiers::SHIFT,
                Heard::Wait,
            ),
        ];
        for (name, key, modifiers, want) in cases {
            assert_eq!(hear(ours(), true, &key, modifiers), want, "{name}");
        }
    }
}
