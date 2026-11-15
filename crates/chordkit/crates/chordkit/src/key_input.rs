//! What a toolkit hands in for a key press, and its normalisation to a chord.

use keyboard_types::Modifiers;

use crate::chord::Chord;
use crate::key::Key;
use crate::modifier::chord_modifiers;
use crate::platform::{Context, Platform};

/// A key press as a toolkit reports it: the modifier flags it holds (as keyboard-types has them,
/// `ALT_GRAPH` included) and the key, which is the logical key with Shift not applied (`z`, not
/// `Z`; `=`, not `+`) as most toolkits' "key without modifiers" gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyInput {
    /// The modifiers held. When AltGr is held, leave Ctrl and Alt as the toolkit has them.
    pub modifiers: Modifiers,
    /// The key pressed.
    pub key: Key,
}

impl KeyInput {
    /// A key press.
    pub fn new(modifiers: Modifiers, key: Key) -> KeyInput {
        KeyInput { modifiers, key }
    }

    /// The key press a toolkit reported as `modifiers` and the logical `key`; `None` when the
    /// key is composed text of more than one character, which is never a chord.
    pub fn from_logical(modifiers: Modifiers, key: &keyboard_types::Key) -> Option<KeyInput> {
        Key::from_logical(key).map(|key| KeyInput::new(modifiers, key))
    }

    /// Whether the press types a character rather than triggering a shortcut: AltGr is held.
    /// Such a press never resolves and is never in the Primary form.
    pub fn types_text(&self) -> bool {
        self.modifiers.contains(Modifiers::ALT_GRAPH)
    }

    /// The canonical chord: the Meta flag folds into Super (on every platform, so `platform` is
    /// kept only for callers that pass one), letters are lower-cased, and a symbol Shift makes is written without Shift.
    /// With AltGr held, Ctrl and Alt are dropped (Windows adds them to AltGr), so the chord can
    /// never match a Ctrl or Alt binding. This is the one place toolkit differences are absorbed.
    pub fn normalise(&self, _platform: Platform) -> Chord {
        self.chord(None)
    }

    /// Like `normalise`, and where `context` has an alias for Primary on `platform`
    /// (`Platform::primary_alias`), the alias modifiers are written as the Primary form: on our
    /// desktop in a terminal, Ctrl+Shift+D is Super+D.
    pub fn normalise_in(&self, platform: Platform, context: Context) -> Chord {
        self.chord(platform.primary_alias(context))
    }

    fn chord(&self, alias: Option<Modifiers>) -> Chord {
        let held = chord_modifiers(self.modifiers);
        let held = if self.types_text() {
            held.difference(Modifiers::CONTROL | Modifiers::ALT)
        } else {
            held
        };
        let modifiers = match alias {
            Some(alias) if held.contains(alias) && !held.contains(Modifiers::META) => {
                held.difference(alias).union(Modifiers::META)
            }
            _ => held,
        };
        let key = match self.key {
            Key::Char(c) => Key::character(c),
            named @ Key::Named(_) => named,
        };
        Chord::new(modifiers, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyboard_types::NamedKey;

    type Case = (
        &'static str,
        Platform,
        &'static [Modifiers],
        Key,
        &'static str,
    );

    #[test]
    fn toolkit_input_normalises_per_platform() {
        let linux = Platform::Linux {
            desktop: crate::platform::Desktop::Kde,
        };
        let ours = Platform::Linux {
            desktop: crate::platform::Desktop::Ours,
        };
        let cases: &[Case] = &[
            (
                "mac command as super",
                Platform::MacOs,
                &[Modifiers::META],
                Key::Char('c'),
                "Super+C",
            ),
            (
                "mac command as meta",
                Platform::MacOs,
                &[Modifiers::META],
                Key::Char('c'),
                "Super+C",
            ),
            (
                "windows key as meta",
                Platform::Windows,
                &[Modifiers::META],
                Key::Char('s'),
                "Super+S",
            ),
            (
                "qt meta on kde",
                linux,
                &[Modifiers::META, Modifiers::SHIFT],
                Key::Char('s'),
                "Shift+Super+S",
            ),
            (
                "ours super",
                ours,
                &[Modifiers::META],
                Key::Char('c'),
                "Super+C",
            ),
            (
                "ours meta",
                ours,
                &[Modifiers::META],
                Key::Char('c'),
                "Super+C",
            ),
            (
                "upper case letter",
                Platform::Windows,
                &[Modifiers::CONTROL],
                Key::Char('Z'),
                "Ctrl+Z",
            ),
            (
                "shift equals is plus",
                Platform::MacOs,
                &[Modifiers::META, Modifiers::SHIFT],
                Key::Char('='),
                "Super++",
            ),
            (
                "shift slash is question",
                Platform::MacOs,
                &[Modifiers::META, Modifiers::SHIFT],
                Key::Char('/'),
                "Super+?",
            ),
            ("meta tap", linux, &[], Key::Named(NamedKey::Meta), "Super"),
            (
                "named key",
                Platform::Windows,
                &[Modifiers::ALT],
                Key::Named(NamedKey::F4),
                "Alt+F4",
            ),
        ];
        for (name, platform, mods, key, expected) in cases {
            let input = KeyInput::new(mods.iter().copied().collect(), *key);
            let wanted: Chord = expected
                .parse()
                .unwrap_or_else(|e| panic!("{expected}: {e}"));
            assert_eq!(input.normalise(*platform), wanted, "{name}");
        }
    }

    type AliasCase = (
        &'static str,
        Platform,
        Context,
        &'static [Modifiers],
        &'static str,
    );

    #[test]
    fn our_terminal_reads_ctrl_shift_as_command() {
        let ours = Platform::Linux {
            desktop: crate::platform::Desktop::Ours,
        };
        let kde = Platform::Linux {
            desktop: crate::platform::Desktop::Kde,
        };
        let ctrl_shift = &[Modifiers::CONTROL, Modifiers::SHIFT];
        let cases: &[AliasCase] = &[
            (
                "ours terminal alias",
                ours,
                Context::Terminal,
                ctrl_shift,
                "Super+D",
            ),
            (
                "ours terminal keeps alt",
                ours,
                Context::Terminal,
                &[Modifiers::CONTROL, Modifiers::SHIFT, Modifiers::ALT],
                "Alt+Super+D",
            ),
            (
                "ours terminal super",
                ours,
                Context::Terminal,
                &[Modifiers::META],
                "Super+D",
            ),
            (
                "ours terminal plain ctrl",
                ours,
                Context::Terminal,
                &[Modifiers::CONTROL],
                "Ctrl+D",
            ),
            (
                "ours normal no alias",
                ours,
                Context::Normal,
                ctrl_shift,
                "Ctrl+Shift+D",
            ),
            (
                "kde terminal no alias",
                kde,
                Context::Terminal,
                ctrl_shift,
                "Ctrl+Shift+D",
            ),
            (
                "mac terminal no alias",
                Platform::MacOs,
                Context::Terminal,
                ctrl_shift,
                "Ctrl+Shift+D",
            ),
        ];
        for (name, platform, context, mods, expected) in cases {
            let input = KeyInput::new(mods.iter().copied().collect(), Key::Char('d'));
            let wanted: Chord = expected
                .parse()
                .unwrap_or_else(|e| panic!("{expected}: {e}"));
            assert_eq!(input.normalise_in(*platform, *context), wanted, "{name}");
        }
    }

    #[test]
    fn alt_gr_drops_ctrl_and_alt() {
        let input = KeyInput::new(
            [Modifiers::CONTROL, Modifiers::ALT].into_iter().collect(),
            Key::Char('@'),
        );
        assert!(!input.types_text());
        let alt_gr = KeyInput::new(input.modifiers | Modifiers::ALT_GRAPH, input.key);
        assert!(alt_gr.types_text());
        let held = alt_gr.normalise(Platform::Windows);
        assert_eq!(held.modifiers(), Modifiers::empty());
        let plain = input.normalise(Platform::Windows);
        assert!(
            plain.modifiers().contains(Modifiers::CONTROL)
                && plain.modifiers().contains(Modifiers::ALT)
        );
    }
}
