//! A [`Shortcut`] as the chord keymap's portable chord and back. The shortcut is written in Mac terms
//! (Command is [`ShortcutKey::Super`]); the chord keymap says what that is on each platform, so
//! Command here is `Primary` there and the other modifiers stay what they are.

use crate::standard_action::normalized;
use crate::vocab::{Shortcut, ShortcutKey};
use keymap::keyboard_types::{Modifiers, NamedKey};
use keymap::{Chord, DefaultChord, Key};

impl Shortcut {
    /// The portable chord this shortcut is: Command is `Primary`. `None` for no key at all, or
    /// a key the chord keymap has no name for (the Menu key).
    #[must_use]
    pub fn default_chord(&self) -> Option<DefaultChord> {
        let keys = self.keys();
        let primary = keys.contains(&ShortcutKey::Super);
        let modifiers = keys
            .iter()
            .filter_map(|key| modifier_of(*key))
            .fold(Modifiers::empty(), |flags, flag| flags | flag);
        let key = keys.iter().find(|key| key.modifier_rank().is_none())?;
        Some(DefaultChord::new(
            primary,
            Chord::new(modifiers, chord_key(*key)?),
        ))
    }

    /// The shortcut a portable chord is, written in Mac terms; `None` for a key the shortcut
    /// vocabulary has no cap for (`F13` and up, print screen, a tap).
    #[must_use]
    pub fn from_default_chord(chord: DefaultChord) -> Option<Shortcut> {
        let base = chord.base();
        let held = if chord.primary() {
            Modifiers::META
        } else {
            Modifiers::empty()
        };
        let modifiers = SHORTCUT_MODIFIERS
            .iter()
            .filter(|(flag, _)| (base.modifiers() | held).contains(*flag))
            .map(|(_, key)| *key);
        let keys = modifiers.chain([shortcut_key(base.key())?]);
        Some(Shortcut(normalized(keys)))
    }
}

/// Each modifier flag and the shortcut key that is it; the OS key is Command.
const SHORTCUT_MODIFIERS: [(Modifiers, ShortcutKey); 4] = [
    (Modifiers::CONTROL, ShortcutKey::Ctrl),
    (Modifiers::ALT, ShortcutKey::Alt),
    (Modifiers::SHIFT, ShortcutKey::Shift),
    (Modifiers::META, ShortcutKey::Super),
];

fn modifier_of(key: ShortcutKey) -> Option<Modifiers> {
    SHORTCUT_MODIFIERS
        .iter()
        .find(|(_, candidate)| *candidate == key)
        .map(|(flag, _)| *flag)
        .filter(|flag| *flag != Modifiers::META)
}

/// Each named key the chord keymap and the shortcut vocabulary both have.
const NAMED: [(ShortcutKey, NamedKey); 14] = [
    (ShortcutKey::Enter, NamedKey::Enter),
    (ShortcutKey::Escape, NamedKey::Escape),
    (ShortcutKey::Tab, NamedKey::Tab),
    (ShortcutKey::Backspace, NamedKey::Backspace),
    (ShortcutKey::Up, NamedKey::ArrowUp),
    (ShortcutKey::Down, NamedKey::ArrowDown),
    (ShortcutKey::Left, NamedKey::ArrowLeft),
    (ShortcutKey::Right, NamedKey::ArrowRight),
    (ShortcutKey::Home, NamedKey::Home),
    (ShortcutKey::End, NamedKey::End),
    (ShortcutKey::Delete, NamedKey::Delete),
    (ShortcutKey::PageUp, NamedKey::PageUp),
    (ShortcutKey::PageDown, NamedKey::PageDown),
    (ShortcutKey::Insert, NamedKey::Insert),
];

fn chord_key(key: ShortcutKey) -> Option<Key> {
    match key {
        ShortcutKey::Char(c) => Some(Key::character(c)),
        ShortcutKey::Space => Some(Key::SPACE),
        ShortcutKey::F(n) if ShortcutKey::function(n).is_some() => Key::function(n),
        other => NAMED
            .iter()
            .find(|(shortcut, _)| *shortcut == other)
            .map(|(_, named)| Key::Named(*named)),
    }
}

fn shortcut_key(key: Key) -> Option<ShortcutKey> {
    match key {
        Key::SPACE => Some(ShortcutKey::Space),
        Key::Char(c) => Some(ShortcutKey::Char(c)),
        Key::Named(named) => (1..=24)
            .find(|n| Key::function(*n) == Some(Key::Named(named)))
            .and_then(ShortcutKey::function)
            .or_else(|| {
                NAMED
                    .iter()
                    .find(|(_, candidate)| *candidate == named)
                    .map(|(shortcut, _)| *shortcut)
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str) -> DefaultChord {
        text.parse().unwrap_or_else(|e| panic!("{text}: {e}"))
    }

    #[test]
    fn command_is_primary_and_the_other_modifiers_stay() {
        let cases = [
            (
                Shortcut(vec![ShortcutKey::Super, ShortcutKey::Char('K')]),
                "Primary+K",
            ),
            (
                Shortcut(vec![
                    ShortcutKey::Shift,
                    ShortcutKey::Super,
                    ShortcutKey::Char('z'),
                ]),
                "Primary+Shift+Z",
            ),
            (
                Shortcut(vec![ShortcutKey::Ctrl, ShortcutKey::Char('1')]),
                "Ctrl+1",
            ),
            (
                Shortcut(vec![ShortcutKey::Super, ShortcutKey::Char('+')]),
                "Primary++",
            ),
            (Shortcut(vec![ShortcutKey::Enter]), "Enter"),
        ];
        for (shortcut, text) in cases {
            assert_eq!(shortcut.default_chord(), Some(parsed(text)), "{text}");
        }
        assert_eq!(Shortcut::default().default_chord(), None);
        assert_eq!(
            Shortcut(vec![ShortcutKey::ContextMenu]).default_chord(),
            None
        );
    }

    #[test]
    fn a_chord_round_trips_through_a_shortcut() {
        for text in [
            "Primary+Shift+Z",
            "Ctrl+Primary+S",
            "Ctrl+Up",
            "Alt+Primary+Escape",
        ] {
            let chord = parsed(text);
            let shortcut = Shortcut::from_default_chord(chord);
            assert_eq!(
                shortcut.and_then(|s| s.default_chord()),
                Some(chord),
                "{text}"
            );
        }
        assert_eq!(Shortcut::from_default_chord(parsed("F13")), None);
    }

    #[test]
    fn a_function_key_is_a_chord_key_and_back() {
        for n in 1..=12 {
            let shortcut = Shortcut(vec![ShortcutKey::F(n)]);
            let chord = parsed(&format!("F{n}"));
            assert_eq!(shortcut.default_chord(), Some(chord), "F{n}");
            assert_eq!(Shortcut::from_default_chord(chord), Some(shortcut), "F{n}");
        }
        let with_alt = Shortcut(vec![ShortcutKey::Alt, ShortcutKey::F(9)]);
        assert_eq!(with_alt.default_chord(), Some(parsed("Alt+F9")));
        assert_eq!(Shortcut(vec![ShortcutKey::F(13)]).default_chord(), None);
        assert_eq!(ShortcutKey::F(9).glyph(), "F9");
        assert_eq!(Shortcut(vec![ShortcutKey::F(9)]).chord(), "f9");
    }
}
