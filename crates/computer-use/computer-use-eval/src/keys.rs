//! The keymap the fake seat announces: every printable ASCII character on its own key, and the
//! named keys the surfaces answer to.
//!
//! A key code is only a number here, so `decode` turns it back into what it stands for.

use crate::surface::Stroke;
use computer_use_seat_input::{Keycode, Keymap, KeymapEntry, Produces, ShiftLevel};
use keyboard_types::{Key, NamedKey};
use keymap::keyboard_types::Modifiers;

/// The named keys, each with its code.
const NAMED: &[(Key, u32)] = &[
    (Key::Named(NamedKey::Enter), 1),
    (Key::Named(NamedKey::Tab), 2),
    (Key::Named(NamedKey::Backspace), 3),
    (Key::Named(NamedKey::Escape), 4),
    (Key::Named(NamedKey::Delete), 5),
    (Key::Named(NamedKey::Control), 6),
    (Key::Named(NamedKey::Shift), 7),
    (Key::Named(NamedKey::Alt), 8),
    (Key::Named(NamedKey::Meta), 9),
    (Key::Named(NamedKey::ArrowUp), 10),
    (Key::Named(NamedKey::ArrowDown), 11),
    (Key::Named(NamedKey::ArrowLeft), 12),
    (Key::Named(NamedKey::ArrowRight), 13),
    (Key::Named(NamedKey::Home), 14),
    (Key::Named(NamedKey::End), 15),
    (Key::Named(NamedKey::PageUp), 16),
    (Key::Named(NamedKey::PageDown), 17),
];

/// Where the character codes start.
const CHAR_BASE: u32 = 1000;

/// What a key code stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decoded {
    /// A character.
    Char(char),
    /// A key a surface answers to.
    Stroke(Stroke),
    /// A modifier going down or up: held, it makes the next key a shortcut.
    Modifier(Modifiers),
    /// Any other key.
    Other,
}

/// The keymap of the fake seat.
#[must_use]
pub fn keymap() -> Keymap {
    let named = NAMED.iter().map(|(key, code)| KeymapEntry {
        produces: Produces::Named(key.to_string()),
        code: Keycode(*code),
        level: ShiftLevel::Plain,
    });
    let chars = (' '..='~').map(|c| KeymapEntry {
        produces: Produces::Char(c),
        code: Keycode(CHAR_BASE + u32::from(c)),
        level: ShiftLevel::Plain,
    });
    Keymap {
        entries: named.chain(chars).collect(),
    }
}

/// What `code` stands for.
pub fn decode(code: Keycode) -> Decoded {
    if let Some(c) = code
        .0
        .checked_sub(CHAR_BASE)
        .and_then(char::from_u32)
        .filter(|c| (' '..='~').contains(c))
    {
        return Decoded::Char(c);
    }
    match NAMED.iter().find(|(_, n)| *n == code.0).map(|(k, _)| k) {
        Some(Key::Named(NamedKey::Enter)) => Decoded::Stroke(Stroke::Enter),
        Some(Key::Named(NamedKey::Tab)) => Decoded::Stroke(Stroke::Tab),
        Some(Key::Named(NamedKey::Backspace)) => Decoded::Stroke(Stroke::Backspace),
        Some(Key::Named(NamedKey::Escape)) => Decoded::Stroke(Stroke::Escape),
        Some(Key::Named(NamedKey::Control)) => Decoded::Modifier(Modifiers::CONTROL),
        Some(Key::Named(NamedKey::Alt)) => Decoded::Modifier(Modifiers::ALT),
        Some(Key::Named(NamedKey::Shift)) => Decoded::Modifier(Modifiers::SHIFT),
        Some(Key::Named(NamedKey::Meta)) => Decoded::Modifier(Modifiers::META),
        _ => Decoded::Other,
    }
}
