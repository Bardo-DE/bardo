//! A keyboard press or release -> `BlitzKeyEvent`, combining `key_types`' keysym and evdev
//! tables with the modifier state.

use blitz_traits::SmolStr;
use blitz_traits::events::{BlitzKeyEvent, KeyState};
use key_types::{KeyCode, Keysym};
use keyboard_types::{Code, Key, Modifiers};
use smithay_client_toolkit::seat::keyboard::KeyEvent;

/// The Blitz event for `event`: `key` from `Keysym::logical_key`, `code` from
/// `KeyCode::dom_code` of `event.raw_code`, `location` from `Keysym::location`, `text` from
/// the printable part of `event.utf8` (presses only). `repeat`: a compositor/sctk key repeat
/// rather than the first press. `composing`: an IME composition is in progress on the focused
/// surface (`BlitzKeyEvent::is_composing`).
pub(crate) fn key_event(
    event: &KeyEvent,
    mods: Modifiers,
    state: KeyState,
    repeat: bool,
    composing: bool,
) -> BlitzKeyEvent {
    let text = match state {
        KeyState::Pressed => printable(event.utf8.as_deref()).map(SmolStr::from),
        KeyState::Released => None,
    };
    let keysym = Keysym(event.keysym.raw());
    let key = keysym.logical_key(event.utf8.as_deref());
    BlitzKeyEvent {
        key_without_modifiers: unshifted(&key),
        key,
        code: u16::try_from(event.raw_code).map_or(Code::Unidentified, |code| {
            KeyCode::from_code(code).dom_code()
        }),
        modifiers: mods,
        location: keysym.location(),
        is_auto_repeating: repeat,
        is_composing: composing,
        state,
        text,
    }
}

/// `key` with Shift taken off, as far as the keysym tells: a letter is lower case. The layout's
/// unshifted symbol for other keys (`!` on `1`) is not in the keysym, so those stay as they are.
pub(crate) fn unshifted(key: &Key) -> Key {
    match key {
        Key::Character(text) if text.chars().count() == 1 => Key::Character(text.to_lowercase()),
        other => other.clone(),
    }
}

/// `raw`, unless it is empty or contains a control character (Ctrl+C's U+0003 is not text).
fn printable(raw: Option<&str>) -> Option<&str> {
    raw.filter(|t| !t.is_empty() && !t.chars().any(char::is_control))
}

#[cfg(test)]
mod tests;
