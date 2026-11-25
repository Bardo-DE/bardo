//! The keymap the EIS server sent, reduced to what planning needs.

use crate::op::Keycode;
use keyboard_types::{Key, NamedKey};
use serde::{Deserialize, Serialize};
use xkbcommon::xkb::{self, Keysym};

/// An xkb keycode is the evdev code plus eight.
const EVDEV_OFFSET: u32 = 8;

/// Whether a key produces its character with Shift held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShiftLevel {
    /// Without a modifier.
    Plain,
    /// With Shift.
    Shifted,
}

/// What a key produces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Produces {
    /// A character.
    Char(char),
    /// A named key (`Enter`, `Tab`, `Control`, `F5`...), by its W3C `key` value.
    Named(String),
}

/// One row of a keymap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeymapEntry {
    /// What it produces.
    pub produces: Produces,
    /// The key.
    pub code: Keycode,
    /// At which level.
    pub level: ShiftLevel,
}

/// An xkb keymap, parsed once when the seat connects.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Keymap {
    /// The rows, in the keymap's own order.
    pub entries: Vec<KeymapEntry>,
}

/// Why a keymap could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum KeymapError {
    /// The text is not an xkb keymap.
    #[error("not an xkb keymap: {0}")]
    Malformed(String),
}

/// The W3C `key` of the keysyms that are keys of their own and not characters: the writing
/// keys with control codes (Enter, Tab, Backspace, Escape, Delete), the cursor and editing
/// keys, the modifiers and the function keys. A keysym not in the table is a character when it
/// has a Unicode form, and nothing otherwise.
fn named_key(sym: Keysym) -> Option<Key> {
    let raw = sym.raw();
    Some(match raw {
        0xff0d => Key::Named(NamedKey::Enter),
        0xff09 | 0xfe20 => Key::Named(NamedKey::Tab),
        0xff08 => Key::Named(NamedKey::Backspace),
        0xff1b => Key::Named(NamedKey::Escape),
        0xffff => Key::Named(NamedKey::Delete),
        0xff63 => Key::Named(NamedKey::Insert),
        0xff50 => Key::Named(NamedKey::Home),
        0xff57 => Key::Named(NamedKey::End),
        0xff55 => Key::Named(NamedKey::PageUp),
        0xff56 => Key::Named(NamedKey::PageDown),
        0xff51 => Key::Named(NamedKey::ArrowLeft),
        0xff52 => Key::Named(NamedKey::ArrowUp),
        0xff53 => Key::Named(NamedKey::ArrowRight),
        0xff54 => Key::Named(NamedKey::ArrowDown),
        0xffe1 | 0xffe2 => Key::Named(NamedKey::Shift),
        0xffe3 | 0xffe4 => Key::Named(NamedKey::Control),
        0xffe7..=0xffea => Key::Named(NamedKey::Alt),
        0xffeb | 0xffec => Key::Named(NamedKey::Meta),
        0xfe03 => Key::Named(NamedKey::AltGraph),
        0xffe5 => Key::Named(NamedKey::CapsLock),
        0xff7f => Key::Named(NamedKey::NumLock),
        0xff14 => Key::Named(NamedKey::ScrollLock),
        0xff67 => Key::Named(NamedKey::ContextMenu),
        0xff61 => Key::Named(NamedKey::PrintScreen),
        0xff13 => Key::Named(NamedKey::Pause),
        0xffbe => Key::Named(NamedKey::F1),
        0xffbf => Key::Named(NamedKey::F2),
        0xffc0 => Key::Named(NamedKey::F3),
        0xffc1 => Key::Named(NamedKey::F4),
        0xffc2 => Key::Named(NamedKey::F5),
        0xffc3 => Key::Named(NamedKey::F6),
        0xffc4 => Key::Named(NamedKey::F7),
        0xffc5 => Key::Named(NamedKey::F8),
        0xffc6 => Key::Named(NamedKey::F9),
        0xffc7 => Key::Named(NamedKey::F10),
        0xffc8 => Key::Named(NamedKey::F11),
        0xffc9 => Key::Named(NamedKey::F12),
        _ => return None,
    })
}

/// What a keysym produces: a named key, else the character it stands for. Keypad keysyms are
/// left out (their second level is Num Lock, not Shift), and so are dead keys and every keysym
/// that has no Unicode form.
fn produces_of(sym: Keysym) -> Option<Produces> {
    if let Some(key) = named_key(sym) {
        return Some(Produces::Named(key.to_string()));
    }
    if sym.is_keypad_key() {
        return None;
    }
    char::from_u32(xkb::keysym_to_utf32(sym))
        .filter(|c| !c.is_control())
        .map(Produces::Char)
}

/// The rows of one key: what its first layout produces at level one, then at level two. A key
/// the keymap does not name, or one below the evdev range, has none.
fn rows_of_key(keymap: &xkb::Keymap, key: xkb::Keycode) -> Vec<KeymapEntry> {
    let Some(code) = key
        .raw()
        .checked_sub(EVDEV_OFFSET)
        .filter(|_| keymap.key_get_name(key).is_some())
    else {
        return Vec::new();
    };
    [(0, ShiftLevel::Plain), (1, ShiftLevel::Shifted)]
        .into_iter()
        .filter_map(|(index, level)| {
            let produces = produces_of(*keymap.key_get_syms_by_level(key, 0, index).first()?)?;
            Some(KeymapEntry {
                produces,
                code: Keycode(code),
                level,
            })
        })
        .collect()
}

impl Keymap {
    /// Reads the keymap text an EIS server sends on its keyboard device (xkb text format v1)
    /// with libxkbcommon: for every key of the first layout, what level one (plain) and level
    /// two (Shift) produce, as a character or a named key (`Enter`, `Control`, `F5`...). The
    /// codes are evdev codes (an xkb keycode is the evdev code plus 8). Rows come in key code
    /// order, a key's plain row before its shifted one. The parse reads no file and no
    /// environment variable.
    ///
    /// # Errors
    ///
    /// Returns `KeymapError::Malformed` when `text` is not an xkb keymap.
    pub fn parse_xkb(text: &str) -> Result<Keymap, KeymapError> {
        let context =
            xkb::Context::new(xkb::CONTEXT_NO_DEFAULT_INCLUDES | xkb::CONTEXT_NO_ENVIRONMENT_NAMES);
        let keymap = xkb::Keymap::new_from_string(
            &context,
            text.to_owned(),
            xkb::KEYMAP_FORMAT_TEXT_V1,
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .ok_or_else(|| KeymapError::Malformed("libxkbcommon refused the text".to_owned()))?;
        let first = keymap.min_keycode().raw();
        let last = keymap.max_keycode().raw();
        let entries = (first..=last)
            .map(xkb::Keycode::new)
            .flat_map(|key| rows_of_key(&keymap, key))
            .collect();
        Ok(Keymap { entries })
    }

    /// The key that types `c`, and whether Shift is needed.
    #[must_use]
    pub fn stroke_for(&self, c: char) -> Option<(Keycode, ShiftLevel)> {
        self.entries
            .iter()
            .find(|e| e.produces == Produces::Char(c))
            .map(|e| (e.code, e.level))
    }

    /// The key code of a named key or a character key by its W3C `key` value.
    #[must_use]
    pub fn code_of(&self, key: &keyboard_types::Key) -> Option<Keycode> {
        let wanted = match key {
            Key::Character(text) => {
                let mut chars = text.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => Produces::Char(c),
                    _ => return None,
                }
            }
            Key::Named(named) => Produces::Named(named.to_string()),
        };
        // A shifted letter names the same key as its lower case.
        let lower = match &wanted {
            Produces::Char(c) => Some(Produces::Char(c.to_ascii_lowercase())),
            Produces::Named(_) => None,
        };
        let find = |p: &Produces| {
            self.entries
                .iter()
                .find(|e| &e.produces == p && e.level == ShiftLevel::Plain)
                .map(|e| e.code)
        };
        find(&wanted).or_else(|| lower.as_ref().and_then(find))
    }
}
