//! A chordkit chord as a casement chord (pure).

use casement_core::input::{Chord, Keysym};
use casement_protocols::vocab::HeldMods;
use chordkit::keyboard_types::{Modifiers, NamedKey};
use chordkit::Key;

/// `F1`'s keysym; the function keys follow in order.
const F1: u32 = 0xffbe;
/// `F1` to `F24`, in keysym order.
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
/// The first keysym of the Unicode range: `0x0100_0000 + code point`.
const UNICODE_BASE: u32 = 0x0100_0000;

/// The casement chord that `chord` means: Super is Command (the key arrives as the logo key),
/// Alt is Option. `None` for a chord casement cannot see as a key press (a tapped modifier,
/// or a function key past F24).
///
/// chordkit writes `Shift+=` as `+` and `Shift+/` as `?`; the keyboard reports the unshifted
/// key with Shift held, so those two are written back that way.
pub fn chord_of(chord: chordkit::Chord) -> Option<Chord> {
    let (key, shifted) = match chord.key() {
        Key::Char('+') => (Keysym(0x3d), true),
        Key::Char('?') => (Keysym(0x2f), true),
        other => (keysym_of(other)?, false),
    };
    let mods = mods_of(chord.modifiers());
    Some(Chord {
        mods: match shifted {
            true => mods.with(HeldMods::SHIFT),
            false => mods,
        },
        key,
    })
}

fn mods_of(mods: Modifiers) -> HeldMods {
    [
        (Modifiers::CONTROL, HeldMods::CONTROL),
        (Modifiers::ALT, HeldMods::OPTION),
        (Modifiers::SHIFT, HeldMods::SHIFT),
        (Modifiers::META, HeldMods::COMMAND),
    ]
    .into_iter()
    .filter(|(flag, _)| mods.contains(*flag))
    .fold(HeldMods::NONE, |held, (_, one)| held.with(one))
}

fn keysym_of(key: Key) -> Option<Keysym> {
    match key {
        Key::Char(c) => Some(Keysym(match u32::from(c) {
            code @ (0x20..=0x7e | 0xa0..=0xff) => code,
            code => UNICODE_BASE + code,
        })),
        Key::Named(named) => named_keysym(named).map(Keysym),
    }
}

fn named_keysym(key: NamedKey) -> Option<u32> {
    match key {
        NamedKey::Tab => Some(0xff09),
        NamedKey::Enter => Some(0xff0d),
        NamedKey::Escape => Some(0xff1b),
        NamedKey::Backspace => Some(0xff08),
        NamedKey::Delete => Some(0xffff),
        NamedKey::Insert => Some(0xff63),
        NamedKey::Home => Some(0xff50),
        NamedKey::End => Some(0xff57),
        NamedKey::PageUp => Some(0xff55),
        NamedKey::PageDown => Some(0xff56),
        NamedKey::ArrowUp => Some(0xff52),
        NamedKey::ArrowDown => Some(0xff54),
        NamedKey::ArrowLeft => Some(0xff51),
        NamedKey::ArrowRight => Some(0xff53),
        NamedKey::PrintScreen => Some(0xff61),
        other => FUNCTION_KEYS
            .iter()
            .position(|f| *f == other)
            .and_then(|index| u32::try_from(index).ok())
            .map(|index| F1 + index),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Case = (&'static str, &'static str, Option<&'static str>);

    /// (case, chordkit text, casement text)
    const CASES: &[Case] = &[
        ("super is command", "Super+W", Some("cmd+w")),
        ("alt is option", "Alt+Super+H", Some("option+cmd+h")),
        (
            "ctrl and shift",
            "Ctrl+Shift+Super+F",
            Some("ctrl+shift+cmd+f"),
        ),
        ("a digit", "Shift+Super+3", Some("shift+cmd+3")),
        ("space", "Super+Space", Some("cmd+space")),
        ("tab", "Shift+Super+Tab", Some("shift+cmd+Tab")),
        ("escape", "Alt+Super+Escape", Some("option+cmd+Escape")),
        ("an arrow", "Ctrl+Left", Some("ctrl+Left")),
        ("print", "Print", Some("Print")),
        (
            "a function key",
            "Ctrl+Alt+Shift+Super+F19",
            Some("ctrl+option+shift+cmd+F19"),
        ),
        ("a modifier tapped alone", "Super", None),
    ];

    #[test]
    fn chords_read_as_casements_own() {
        for (case, written, want) in CASES {
            let chord: chordkit::Chord = written.parse().expect(case);
            let want = want.map(|text| Chord::parse(text).expect(case));
            assert_eq!(chord_of(chord), want, "{case}");
        }
    }

    #[test]
    fn the_symbols_shift_makes_come_back_as_shifted_keys() {
        let plus = chordkit::Chord::new(Modifiers::META, Key::Char('+'));
        let question = chordkit::Chord::new(Modifiers::META, Key::Char('?'));
        let want = |text| Some(Chord::parse(text).expect("a chord"));
        assert_eq!(chord_of(plus), want("shift+cmd+equal"));
        assert_eq!(chord_of(question), want("shift+cmd+slash"));
    }

    #[test]
    fn a_character_outside_latin_one_is_a_unicode_keysym() {
        let euro = chordkit::Chord::new(Modifiers::empty(), Key::Char('\u{20ac}'));
        assert_eq!(
            chord_of(euro).map(|chord| chord.key),
            Some(Keysym(0x0100_20ac))
        );
    }
}
