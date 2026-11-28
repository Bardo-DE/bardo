//! `KeyCode`: an evdev key code, named through the table.

use crate::names::NAMES;

/// An evdev key code (`KEY_*` in `linux/input-event-codes.h`).
///
/// Every `u16` is a key the kernel may send, so `from_code` accepts any code; the name table
/// covers the standard keyboard, keypad and media keys, and `name` reports `"unknown"` for a
/// code outside it. Code that spells a key in text goes through `named`, so a typo is a parse
/// failure and never a wrong key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyCode(u16);

/// The `name` of a code the table does not list.
const UNNAMED: &str = "unknown";

impl KeyCode {
    /// Escape.
    pub const ESC: KeyCode = KeyCode(1);
    /// Left Ctrl.
    pub const LEFTCTRL: KeyCode = KeyCode(29);
    /// Left Shift.
    pub const LEFTSHIFT: KeyCode = KeyCode(42);
    /// Right Shift.
    pub const RIGHTSHIFT: KeyCode = KeyCode(54);
    /// Left Alt.
    pub const LEFTALT: KeyCode = KeyCode(56);
    /// Right Ctrl.
    pub const RIGHTCTRL: KeyCode = KeyCode(97);
    /// Right Alt.
    pub const RIGHTALT: KeyCode = KeyCode(100);
    /// The left arrow.
    pub const LEFT: KeyCode = KeyCode(105);
    /// The right arrow.
    pub const RIGHT: KeyCode = KeyCode(106);
    /// Left Super (the Command key on a Mac keyboard).
    pub const LEFTMETA: KeyCode = KeyCode(125);
    /// Right Super.
    pub const RIGHTMETA: KeyCode = KeyCode(126);
    /// F24, the last function key the table names.
    pub const F24: KeyCode = KeyCode(194);

    /// The key a device reported as `code`; any code is a key.
    #[must_use]
    pub const fn from_code(code: u16) -> KeyCode {
        KeyCode(code)
    }

    /// The key called `name` (lowercase, as in `KEY_<NAME>`), if the table has it.
    #[must_use]
    pub fn named(name: &str) -> Option<KeyCode> {
        NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|&(_, c)| KeyCode(c))
    }

    /// The table name of this key, or `"unknown"` for a code the table does not list.
    #[must_use]
    pub fn name(self) -> &'static str {
        NAMES
            .iter()
            .find(|&&(_, c)| c == self.0)
            .map_or(UNNAMED, |&(n, _)| n)
    }

    /// The DOM physical key (`keyboard_types::Code`) this key is, whatever the layout:
    /// `Code::Unidentified` for a code the table does not know.
    #[must_use]
    pub fn dom_code(self) -> keyboard_types::Code {
        crate::dom::code_from_evdev(u32::from(self.0))
    }

    /// The evdev code.
    #[must_use]
    pub const fn code(self) -> u16 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::names::NAMES;

    #[test]
    fn names_resolve_to_known_codes() {
        const CASES: &[(&str, u16)] = &[
            ("esc", 1),
            ("tab", 15),
            ("a", 30),
            ("grave", 41),
            ("space", 57),
            ("f4", 62),
            ("home", 102),
            ("left", 105),
            ("end", 107),
            ("delete", 111),
            ("backspace", 14),
            ("leftmeta", 125),
        ];
        for &(name, code) in CASES {
            assert_eq!(
                KeyCode::named(name),
                Some(KeyCode::from_code(code)),
                "{name}"
            );
            assert_eq!(KeyCode::from_code(code).name(), name, "{code}");
        }
    }

    #[test]
    fn constants_match_their_names() {
        const CASES: &[(KeyCode, &str)] = &[
            (KeyCode::ESC, "esc"),
            (KeyCode::LEFTCTRL, "leftctrl"),
            (KeyCode::RIGHTCTRL, "rightctrl"),
            (KeyCode::LEFTALT, "leftalt"),
            (KeyCode::RIGHTALT, "rightalt"),
            (KeyCode::LEFTMETA, "leftmeta"),
            (KeyCode::RIGHTMETA, "rightmeta"),
            (KeyCode::LEFTSHIFT, "leftshift"),
            (KeyCode::RIGHTSHIFT, "rightshift"),
        ];
        for &(key, name) in CASES {
            assert_eq!(key.name(), name);
            assert_eq!(KeyCode::named(name), Some(key), "{name}");
        }
    }

    #[test]
    fn names_are_unique_lowercase_tokens_with_unique_codes() {
        for (i, (n, c)) in NAMES.iter().enumerate() {
            assert!(
                NAMES[i + 1..].iter().all(|(m, d)| m != n && d != c),
                "{n} is not unique"
            );
            assert!(
                n.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
                "{n} is not a lowercase token"
            );
        }
    }

    #[test]
    fn table_covers_the_standard_keyboard() {
        const CASES: &[&str] = &[
            "q",
            "0",
            "f1",
            "f12",
            "f13",
            "f24",
            "minus",
            "semicolon",
            "kp0",
            "kpenter",
            "volumeup",
            "mute",
            "playpause",
            "brightnessup",
            "capslock",
            "rightmeta",
            "compose",
            "sysrq",
        ];
        for name in CASES {
            assert!(KeyCode::named(name).is_some(), "{name}");
        }
    }

    #[test]
    fn unlisted_code_is_a_key_without_a_name() {
        let key = KeyCode::from_code(0xffff);
        assert_eq!(key.code(), 0xffff);
        assert_eq!(key.name(), "unknown");
        assert_eq!(KeyCode::named("unknown"), None);
        assert_eq!(KeyCode::named("A"), None);
    }
}
