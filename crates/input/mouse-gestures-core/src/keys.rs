//! Key combinations for the virtual keyboard (O3), written in settings as `"alt+left"`.
//!
//! A `Key` can only come from the name table below, so every key the daemon may press is one
//! it knows the evdev code of; the keyboard device registers exactly the keys in use.

use key_types::{KeyCode, Modifier};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Whether holding `modifier` makes the next key a shortcut. Shift alone does not.
fn makes_a_shortcut(modifier: Modifier) -> bool {
    matches!(modifier, Modifier::Ctrl | Modifier::Alt | Modifier::Super)
}

/// A non-modifier key, by evdev code; constructible only from `NAMES`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(KeyCode);

/// Every key name the settings file may use, with its evdev code.
#[rustfmt::skip]
pub const NAMES: &[(&str, u16)] = &[
    ("escape", 1), ("1", 2), ("2", 3), ("3", 4), ("4", 5), ("5", 6), ("6", 7), ("7", 8),
    ("8", 9), ("9", 10), ("0", 11), ("minus", 12), ("equal", 13), ("backspace", 14),
    ("tab", 15), ("q", 16), ("w", 17), ("e", 18), ("r", 19), ("t", 20), ("y", 21), ("u", 22),
    ("i", 23), ("o", 24), ("p", 25), ("bracketleft", 26), ("bracketright", 27), ("enter", 28),
    ("a", 30), ("s", 31), ("d", 32), ("f", 33), ("g", 34), ("h", 35), ("j", 36), ("k", 37),
    ("l", 38), ("z", 44), ("x", 45), ("c", 46), ("v", 47), ("b", 48), ("n", 49), ("m", 50),
    ("comma", 51), ("period", 52), ("slash", 53), ("space", 57), ("f1", 59), ("f2", 60),
    ("f3", 61), ("f4", 62), ("f5", 63), ("f6", 64), ("f7", 65), ("f8", 66), ("f9", 67),
    ("f10", 68), ("f11", 87), ("f12", 88), ("home", 102), ("up", 103), ("pageup", 104),
    ("left", 105), ("right", 106), ("end", 107), ("down", 108), ("pagedown", 109),
    ("insert", 110), ("delete", 111), ("back", 158), ("forward", 159),
];

impl Key {
    /// The left arrow.
    pub const LEFT: Key = Key(KeyCode::LEFT);
    /// The right arrow.
    pub const RIGHT: Key = Key(KeyCode::RIGHT);

    /// The evdev code.
    #[must_use]
    pub fn code(self) -> u16 {
        self.0.code()
    }

    fn parse(s: &str) -> Option<Self> {
        NAMES
            .iter()
            .find(|(n, _)| *n == s)
            .map(|&(_, c)| Key(KeyCode::from_code(c)))
    }

    /// Whether the key is sent only together with ctrl, alt or super: a letter, digit,
    /// punctuation, Enter, Space, Tab, Backspace, Delete or Escape. The input service refuses
    /// them on their own so that mouse gestures daemon cannot type text. Navigation, function keys, Insert,
    /// Back and Forward go out freely. The service keeps the same table.
    #[must_use]
    pub fn needs_a_shortcut_modifier(self) -> bool {
        !matches!(self.0.code(), 59..=68 | 87 | 88 | 102..=110 | 158 | 159)
    }

    fn name(self) -> &'static str {
        NAMES
            .iter()
            .find(|&&(_, c)| c == self.0.code())
            .map_or("?", |(n, _)| n)
    }
}

/// Modifiers plus one key, pressed in order and released in reverse.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct KeyCombo {
    pub mods: Vec<Modifier>,
    pub key: Key,
}

impl KeyCombo {
    /// Alt+Left: Back in every browser and file manager.
    #[must_use]
    pub fn alt(key: Key) -> Self {
        Self {
            mods: vec![Modifier::Alt],
            key,
        }
    }

    /// Whether the input service will send this combo: a key that types text must come with
    /// ctrl, alt or super.
    #[must_use]
    pub fn is_sendable(&self) -> bool {
        !self.key.needs_a_shortcut_modifier() || self.mods.iter().any(|m| makes_a_shortcut(*m))
    }

    /// Every evdev code this combo presses.
    pub fn codes(&self) -> impl Iterator<Item = u16> + '_ {
        self.mods
            .iter()
            .map(|m| m.key().code())
            .chain(std::iter::once(self.key.code()))
    }
}

impl TryFrom<String> for KeyCombo {
    /// Why a combo string is refused: it names an unknown key or modifier.
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        let lower = s.trim().to_ascii_lowercase();
        let parts: Vec<&str> = lower.split('+').map(str::trim).collect();
        let err = || format!("{s:?} is not a key combination like \"alt+left\"");
        let (key, mods) = parts.split_last().ok_or_else(err)?;
        let mods = mods
            .iter()
            .map(|m| Modifier::named(m))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(err)?;
        Ok(Self {
            mods,
            key: Key::parse(key).ok_or_else(err)?,
        })
    }
}

impl From<KeyCombo> for String {
    fn from(k: KeyCombo) -> String {
        k.to_string()
    }
}

impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for m in &self.mods {
            write!(f, "{}+", m.name())?;
        }
        f.write_str(self.key.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_table() {
        const CASES: &[(&str, Option<&str>)] = &[
            ("alt+left", Some("alt+left")),
            ("Alt + Right", Some("alt+right")),
            ("ctrl+shift+t", Some("ctrl+shift+t")),
            ("super+pageup", Some("super+pageup")),
            ("f5", Some("f5")),
            ("alt+", None),
            ("hyper+left", None),
            ("alt+leftish", None),
            ("", None),
        ];
        for &(input, want) in CASES {
            let got = KeyCombo::try_from(input.to_owned())
                .ok()
                .map(|k| k.to_string());
            assert_eq!(got.as_deref(), want, "{input:?}");
        }
    }

    #[test]
    fn codes_press_modifiers_first() {
        let k = KeyCombo::alt(Key::LEFT);
        assert_eq!(k.codes().collect::<Vec<_>>(), [56, 105]);
    }

    #[test]
    fn only_a_shortcut_can_press_a_key_that_types() {
        const CASES: &[(&str, bool)] = &[
            ("alt+left", true),
            ("f5", true),
            ("shift+f5", true),
            ("home", true),
            ("back", true),
            ("ctrl+shift+t", true),
            ("super+enter", true),
            ("ctrl+alt+delete", true),
            ("t", false),
            ("shift+t", false),
            ("enter", false),
            ("space", false),
            ("tab", false),
            ("shift+tab", false),
            ("backspace", false),
            ("delete", false),
            ("escape", false),
            ("minus", false),
            ("5", false),
            ("shift+insert", true),
        ];
        for &(input, want) in CASES {
            let combo = KeyCombo::try_from(input.to_owned()).unwrap();
            assert_eq!(combo.is_sendable(), want, "{input}");
        }
    }

    #[test]
    fn the_keys_that_type_are_exactly_the_ones_outside_navigation_and_function_keys() {
        let free: Vec<&str> = NAMES
            .iter()
            .filter(|&&(_, c)| !Key(KeyCode::from_code(c)).needs_a_shortcut_modifier())
            .map(|&(n, _)| n)
            .collect();
        assert_eq!(
            free,
            [
                "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9", "f10", "f11", "f12", "home",
                "up", "pageup", "left", "right", "end", "down", "pagedown", "insert", "back",
                "forward"
            ]
        );
    }

    #[test]
    fn names_are_unique() {
        for (i, (n, c)) in NAMES.iter().enumerate() {
            assert!(NAMES[i + 1..].iter().all(|(m, d)| m != n && d != c), "{n}");
        }
    }
}
