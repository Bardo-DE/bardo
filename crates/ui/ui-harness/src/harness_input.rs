//! The harness's input vocabulary: the UI kit's keys and pointer buttons as the Blitz events a window
//! would deliver for them.

use crate::input::PasteChord;
use blitz_traits::events::{
    BlitzPointerEvent, BlitzPointerId, MouseEventButton, MouseEventButtons, PointerCoords,
};
use keyboard_types::{Code, Key as DomKey, Modifiers, NamedKey};
use keymap::{Context as KeyContext, Platform};
use ui_components::prelude::*;
use ui_core::press::PointerButton;

/// The Blitz button, and the held-buttons set while it is down, for a Bardo pointer button.
pub(crate) fn blitz_button(button: PointerButton) -> (MouseEventButton, MouseEventButtons) {
    match button {
        PointerButton::Primary => (MouseEventButton::Main, MouseEventButtons::Primary),
        PointerButton::Secondary => (MouseEventButton::Secondary, MouseEventButtons::Secondary),
        PointerButton::Middle => (MouseEventButton::Auxiliary, MouseEventButtons::Auxiliary),
    }
}

/// The pointer buttons the harness's mouse holds down: a move between a press and its release
/// carries them, so a drag is a drag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HeldButtons(MouseEventButtons);

impl HeldButtons {
    /// Whether `button` is down.
    #[must_use]
    pub fn contains(self, button: PointerButton) -> bool {
        self.0.contains(blitz_button(button).1)
    }

    /// Whether none is down.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.0.is_empty()
    }

    /// These, with `button` down too.
    pub(crate) fn with(self, button: PointerButton) -> Self {
        HeldButtons(self.0 | blitz_button(button).1)
    }

    /// These, with `button` up.
    pub(crate) fn without(self, button: PointerButton) -> Self {
        HeldButtons(self.0 - blitz_button(button).1)
    }

    /// Blitz's set.
    pub(crate) fn blitz(self) -> MouseEventButtons {
        self.0
    }
}

/// A mouse pointer event at `at`, for `button`, with `buttons` held and `mods` down.
pub(crate) fn pointer(
    at: Point,
    button: MouseEventButton,
    buttons: MouseEventButtons,
    mods: Modifiers,
) -> BlitzPointerEvent {
    let (x, y) = (at.x.0, at.y.0);
    BlitzPointerEvent {
        id: BlitzPointerId::Mouse,
        is_primary: true,
        coords: PointerCoords {
            page_x: x,
            page_y: y,
            screen_x: x,
            screen_y: y,
            client_x: x,
            client_y: y,
        },
        button,
        buttons,
        mods,
        details: blitz_traits::events::PointerDetails::default(),
        element: blitz_traits::events::Point::<f32>::default(),
        active_pointers: std::sync::Arc::default(),
    }
}

/// The DOM key and physical code for a Bardo key.
pub(crate) fn keyboard(key: ShortcutKey) -> (DomKey, Code) {
    match key {
        ShortcutKey::Ctrl => (DomKey::Named(NamedKey::Control), Code::ControlLeft),
        ShortcutKey::Shift => (DomKey::Named(NamedKey::Shift), Code::ShiftLeft),
        ShortcutKey::Alt => (DomKey::Named(NamedKey::Alt), Code::AltLeft),
        ShortcutKey::Super => (DomKey::Named(NamedKey::Meta), Code::MetaLeft),
        ShortcutKey::Char(c) => (DomKey::Character(c.to_string()), letter(c)),
        ShortcutKey::Space => (DomKey::Character(" ".into()), Code::Space),
        ShortcutKey::Enter => (DomKey::Named(NamedKey::Enter), Code::Enter),
        ShortcutKey::Escape => (DomKey::Named(NamedKey::Escape), Code::Escape),
        ShortcutKey::Tab => (DomKey::Named(NamedKey::Tab), Code::Tab),
        ShortcutKey::Backspace => (DomKey::Named(NamedKey::Backspace), Code::Backspace),
        ShortcutKey::Up => (DomKey::Named(NamedKey::ArrowUp), Code::ArrowUp),
        ShortcutKey::Down => (DomKey::Named(NamedKey::ArrowDown), Code::ArrowDown),
        ShortcutKey::Left => (DomKey::Named(NamedKey::ArrowLeft), Code::ArrowLeft),
        ShortcutKey::Right => (DomKey::Named(NamedKey::ArrowRight), Code::ArrowRight),
        ShortcutKey::Home => (DomKey::Named(NamedKey::Home), Code::Home),
        ShortcutKey::End => (DomKey::Named(NamedKey::End), Code::End),
        ShortcutKey::Delete => (DomKey::Named(NamedKey::Delete), Code::Delete),
        ShortcutKey::PageUp => (DomKey::Named(NamedKey::PageUp), Code::PageUp),
        ShortcutKey::PageDown => (DomKey::Named(NamedKey::PageDown), Code::PageDown),
        ShortcutKey::Insert => (DomKey::Named(NamedKey::Insert), Code::Insert),
        ShortcutKey::ContextMenu => (DomKey::Named(NamedKey::ContextMenu), Code::ContextMenu),
        ShortcutKey::F(n) => function_keyboard(n),
    }
}

/// `F1` to `F12`; any other number is an unidentified key.
fn function_keyboard(n: u8) -> (DomKey, Code) {
    let keys = [
        (DomKey::Named(NamedKey::F1), Code::F1),
        (DomKey::Named(NamedKey::F2), Code::F2),
        (DomKey::Named(NamedKey::F3), Code::F3),
        (DomKey::Named(NamedKey::F4), Code::F4),
        (DomKey::Named(NamedKey::F5), Code::F5),
        (DomKey::Named(NamedKey::F6), Code::F6),
        (DomKey::Named(NamedKey::F7), Code::F7),
        (DomKey::Named(NamedKey::F8), Code::F8),
        (DomKey::Named(NamedKey::F9), Code::F9),
        (DomKey::Named(NamedKey::F10), Code::F10),
        (DomKey::Named(NamedKey::F11), Code::F11),
        (DomKey::Named(NamedKey::F12), Code::F12),
    ];
    usize::from(n)
        .checked_sub(1)
        .and_then(|at| keys.get(at).cloned())
        .unwrap_or((DomKey::Named(NamedKey::Unidentified), Code::Unidentified))
}

/// The modifier flag a held Bardo key sets; any other key sets none.
pub(crate) fn modifier(key: ShortcutKey) -> Modifiers {
    match key {
        ShortcutKey::Ctrl => Modifiers::CONTROL,
        ShortcutKey::Shift => Modifiers::SHIFT,
        ShortcutKey::Alt => Modifiers::ALT,
        ShortcutKey::Super => Modifiers::META,
        _ => Modifiers::empty(),
    }
}

/// The key, physical code and modifiers a paste chord presses.
pub(crate) fn paste_keys(chord: PasteChord, platform: Platform) -> (DomKey, Code, Modifiers) {
    match chord {
        PasteChord::Primary => (
            DomKey::Character("v".into()),
            Code::KeyV,
            platform.primary(KeyContext::Normal),
        ),
        PasteChord::CtrlV => (
            DomKey::Character("v".into()),
            Code::KeyV,
            Modifiers::CONTROL,
        ),
        PasteChord::CtrlShiftV => (
            DomKey::Character("V".into()),
            Code::KeyV,
            Modifiers::CONTROL | Modifiers::SHIFT,
        ),
        PasteChord::SuperV => (DomKey::Character("v".into()), Code::KeyV, Modifiers::META),
        PasteChord::ShiftInsert => (
            DomKey::Named(NamedKey::Insert),
            Code::Insert,
            Modifiers::SHIFT,
        ),
    }
}

/// The physical key a US layout types `c` with, where it is a letter or a digit.
fn letter(c: char) -> Code {
    format!("Key{}", c.to_ascii_uppercase())
        .parse()
        .or_else(|_| format!("Digit{c}").parse())
        .unwrap_or(Code::Unidentified)
}

#[cfg(test)]
mod tests {
    use super::{keyboard, letter, modifier};
    use keyboard_types::Code;
    use ui_components::prelude::*;

    const LETTERS: &[(char, Code)] = &[
        ('a', Code::KeyA),
        ('Z', Code::KeyZ),
        ('7', Code::Digit7),
        ('/', Code::Unidentified),
    ];

    #[test]
    fn letters_map_to_their_physical_key() {
        for &(c, code) in LETTERS {
            assert_eq!(letter(c), code, "{c:?}");
        }
    }

    #[test]
    fn held_keys_are_modifier_flags() {
        let cases = [
            (ShortcutKey::Ctrl, keyboard_types::Modifiers::CONTROL),
            (ShortcutKey::Alt, keyboard_types::Modifiers::ALT),
            (ShortcutKey::Char('k'), keyboard_types::Modifiers::empty()),
        ];
        for (key, want) in cases {
            assert_eq!(modifier(key), want, "{key:?}");
        }
    }

    #[test]
    fn escape_is_the_named_key() {
        assert_eq!(
            keyboard(ShortcutKey::Escape),
            (keyboard_types::Key::Named(keyboard_types::NamedKey::Escape), Code::Escape)
        );
    }
}
