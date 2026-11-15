//! Chords: modifiers plus a key. `Chord` is concrete (what the keyboard sends);
//! `DefaultChord` may say `Primary`, which each platform resolves.

use keyboard_types::Modifiers;

use crate::key::Key;
use crate::modifier::chord_modifiers;
use crate::platform::{Context, Platform};

/// A concrete chord: physical modifiers plus a key, already canonical.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord {
    modifiers: Modifiers,
    key: Key,
}

impl Chord {
    /// A chord, made canonical: flags other than Ctrl, Alt, Shift and Meta (the OS key) are
    /// dropped; a tapped modifier key carries no other modifier; and the symbols Shift makes on a US layout (`Shift+=` is `+`, `Shift+/` is
    /// `?`, `Shift+-` is `_`) are written as the symbol with no Shift, as menus write them.
    pub fn new(modifiers: Modifiers, key: Key) -> Chord {
        let modifiers = chord_modifiers(modifiers);
        let shifted = modifiers.contains(Modifiers::SHIFT);
        match (key, shifted) {
            (tap, _) if tap.is_tap() => Chord {
                modifiers: Modifiers::empty(),
                key: tap,
            },
            (Key::Char('='), true) => Chord {
                modifiers: modifiers.difference(Modifiers::SHIFT),
                key: Key::Char('+'),
            },
            (Key::Char('/'), true) => Chord {
                modifiers: modifiers.difference(Modifiers::SHIFT),
                key: Key::Char('?'),
            },
            (Key::Char('-'), true) => Chord {
                modifiers: modifiers.difference(Modifiers::SHIFT),
                key: Key::Char('_'),
            },
            (Key::Char('+' | '?' | '_'), true) => Chord {
                modifiers: modifiers.difference(Modifiers::SHIFT),
                key,
            },
            _ => Chord { modifiers, key },
        }
    }

    /// The modifiers held.
    pub fn modifiers(self) -> Modifiers {
        self.modifiers
    }

    /// The key.
    pub fn key(self) -> Key {
        self.key
    }

    /// Whether a text field wants this chord: a typing or editing key with no Ctrl, Alt or
    /// Super (Shift alone still types).
    pub fn belongs_to_text_field(self) -> bool {
        let command = Modifiers::CONTROL | Modifiers::ALT | Modifiers::META;
        self.key.belongs_to_text_field() && !self.modifiers.intersects(command)
    }

    /// Whether a program in a terminal keeps this chord: Ctrl without Shift or Super.
    pub fn is_plain_ctrl(self) -> bool {
        self.modifiers.contains(Modifiers::CONTROL)
            && !self.modifiers.contains(Modifiers::SHIFT)
            && !self.modifiers.contains(Modifiers::META)
    }

    /// Whether this is Ctrl alone with `+`, `=`, `-` or `0`: the window's text size even in a
    /// terminal, as GNOME Terminal, Konsole and Windows Terminal take them.
    pub fn is_text_size(self) -> bool {
        self.modifiers == Modifiers::CONTROL && matches!(self.key, Key::Char('+' | '=' | '-' | '0'))
    }

    /// This chord with `+` for `=`, when it has `=` and no Shift: a chord written with `+` (⌘+,
    /// Ctrl++) is also pressed on the `=` key without Shift, as menus label it and apps take it.
    pub(crate) fn plus_for_equals(self) -> Option<Chord> {
        (self.key == Key::Char('=') && !self.modifiers.contains(Modifiers::SHIFT))
            .then(|| Chord::new(self.modifiers, Key::Char('+')))
    }
}

/// A chord as defaults and overrides write it: portable, possibly using `Primary`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefaultChord {
    primary: bool,
    base: Chord,
}

impl DefaultChord {
    /// A default chord: `base` plus, when `primary`, the platform's primary modifier (Command on
    /// a Mac, Ctrl elsewhere); otherwise it is exactly its physical modifiers.
    pub fn new(primary: bool, base: Chord) -> DefaultChord {
        DefaultChord { primary, base }
    }

    /// Whether the primary modifier is part of it.
    pub fn primary(self) -> bool {
        self.primary
    }

    /// The physical part.
    pub fn base(self) -> Chord {
        self.base
    }

    /// The concrete chord on `platform` in `context`. In a terminal where Primary is Ctrl+Shift,
    /// a chord that already uses Shift cannot add it, so it takes Alt as well (Primary+Shift+Z
    /// is Ctrl+Shift+Alt+Z); `None` when it uses Alt too, which leaves no stand-in.
    pub fn resolve(self, platform: Platform, context: Context) -> Option<Chord> {
        if !self.primary {
            return Some(self.base);
        }
        let primary = platform.primary(context);
        let base = self.base.modifiers();
        let shift_clash = context == Context::Terminal
            && primary.contains(Modifiers::SHIFT)
            && base.contains(Modifiers::SHIFT);
        let modifiers = match (shift_clash, base.contains(Modifiers::ALT)) {
            (false, _) => base.union(primary),
            (true, false) => base.union(primary).union(Modifiers::ALT),
            (true, true) => return None,
        };
        Some(Chord::new(modifiers, self.base.key()))
    }
}

impl From<Chord> for DefaultChord {
    fn from(chord: Chord) -> DefaultChord {
        DefaultChord::new(false, chord)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::Desktop;
    use keyboard_types::NamedKey;

    fn mods(list: &[Modifiers]) -> Modifiers {
        list.iter().copied().collect()
    }

    #[test]
    fn canonical_form_drops_lock_flags_and_shifted_symbols() {
        let meta_z = Chord::new(Modifiers::META | Modifiers::NUM_LOCK, Key::Char('z'));
        assert_eq!(meta_z.modifiers(), Modifiers::META);
        let plus = Chord::new(
            mods(&[Modifiers::SHIFT, Modifiers::CONTROL]),
            Key::Char('='),
        );
        assert_eq!(
            plus,
            Chord::new(mods(&[Modifiers::CONTROL]), Key::Char('+'))
        );
        let tap = Chord::new(mods(&[Modifiers::CONTROL]), Key::Named(NamedKey::Meta));
        assert_eq!(
            tap,
            Chord::new(Modifiers::empty(), Key::Named(NamedKey::Meta))
        );
    }

    #[test]
    fn primary_resolves_per_platform_and_terminal_stands_in_for_shift_clashes() {
        let undo = DefaultChord::new(
            true,
            Chord::new(Modifiers::empty(), Key::Char('z')),
        );
        let redo = DefaultChord::new(
            true,
            Chord::new(mods(&[Modifiers::SHIFT]), Key::Char('z')),
        );
        let windows = Platform::Windows;
        let ours = Platform::Linux {
            desktop: Desktop::Ours,
        };
        assert_eq!(
            undo.resolve(windows, Context::Normal),
            Some(Chord::new(mods(&[Modifiers::CONTROL]), Key::Char('z')))
        );
        assert_eq!(
            undo.resolve(windows, Context::Terminal),
            Some(Chord::new(
                mods(&[Modifiers::CONTROL, Modifiers::SHIFT]),
                Key::Char('z')
            ))
        );
        assert_eq!(
            redo.resolve(windows, Context::Terminal),
            Some(Chord::new(
                mods(&[Modifiers::CONTROL, Modifiers::SHIFT, Modifiers::ALT]),
                Key::Char('z')
            ))
        );
        let paste_plain = DefaultChord::new(
            true,
            Chord::new(mods(&[Modifiers::SHIFT, Modifiers::ALT]), Key::Char('v')),
        );
        assert_eq!(paste_plain.resolve(windows, Context::Terminal), None);
        assert_eq!(
            redo.resolve(ours, Context::Terminal),
            Some(Chord::new(
                mods(&[Modifiers::SHIFT, Modifiers::META]),
                Key::Char('z')
            ))
        );
    }

    #[test]
    fn text_fields_and_terminals_claim_their_keys() {
        let plain = Chord::new(Modifiers::empty(), Key::Char('j'));
        let shifted = Chord::new(mods(&[Modifiers::SHIFT]), Key::Named(NamedKey::Tab));
        let command = Chord::new(mods(&[Modifiers::META]), Key::Char('j'));
        let escape = Chord::new(Modifiers::empty(), Key::Named(NamedKey::Escape));
        assert!(plain.belongs_to_text_field() && shifted.belongs_to_text_field());
        assert!(!command.belongs_to_text_field() && !escape.belongs_to_text_field());
        assert!(Chord::new(mods(&[Modifiers::CONTROL]), Key::Char('c')).is_plain_ctrl());
        assert!(
            !Chord::new(
                mods(&[Modifiers::CONTROL, Modifiers::SHIFT]),
                Key::Char('c')
            )
            .is_plain_ctrl()
        );
    }
}
