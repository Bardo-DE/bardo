//! A key press as the chord keymap reads it: the one place a toolkit's modifiers and key become a
//! [`keymap::KeyInput`]. Everything that resolves a key to an action goes through it, so no
//! other file decides what Ctrl, Super or Meta mean.

use keyboard_types::{Key, Modifiers};
use keymap::{Key as ChordKey, KeyInput};

/// The chord flags `held` reports: Ctrl, Alt, Shift, the OS key (`META`) and `AltGraph`. Locks
/// and the like are left out; `AltGraph` is kept for [`key_input`].
fn flags_of(held: Modifiers) -> Modifiers {
    held & (Modifiers::CONTROL
        | Modifiers::ALT
        | Modifiers::SHIFT
        | Modifiers::META
        | Modifiers::ALT_GRAPH)
}

/// The chord modifiers `held` reports (Ctrl, Alt, Shift and the OS key). Lock and `AltGraph`
/// flags are not chord modifiers.
#[must_use]
pub fn modifiers_of(held: Modifiers) -> Modifiers {
    flags_of(held).difference(Modifiers::ALT_GRAPH)
}

/// The chord key `key` is, or `None` for a multi-character string from an IME.
#[must_use]
pub fn chord_key_of(key: &Key) -> Option<ChordKey> {
    ChordKey::from_logical(key)
}

/// The key press `key` with `modifiers` held, as the chord keymap takes it.
///
/// The key is as the toolkit reports it: a shifted symbol (`!` for Shift+1) is not unshifted here,
/// so an app chord that holds Shift with a symbol key reads the base key from the physical code.
/// `AltGraph` rides in the flags: it types a character, and the chord keymap then reads no chord from the
/// press.
#[must_use]
pub fn key_input(key: &Key, modifiers: Modifiers) -> Option<KeyInput> {
    KeyInput::from_logical(flags_of(modifiers), key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyboard_types::NamedKey;
    use keymap::{Chord, Desktop, Platform};

    type Case = (
        &'static str,
        Platform,
        Modifiers,
        &'static str,
        &'static str,
    );

    fn ours() -> Platform {
        Platform::Linux {
            desktop: Desktop::Ours,
        }
    }

    fn kde() -> Platform {
        Platform::Linux {
            desktop: Desktop::Kde,
        }
    }

    #[test]
    fn super_meta_and_ctrl_normalise_per_platform() {
        let cases: &[Case] = &[
            (
                "mac super",
                Platform::MacOs,
                Modifiers::META,
                "c",
                "Super+C",
            ),
            ("mac meta", Platform::MacOs, Modifiers::META, "c", "Super+C"),
            (
                "mac ctrl",
                Platform::MacOs,
                Modifiers::CONTROL,
                "c",
                "Ctrl+C",
            ),
            ("ours super", ours(), Modifiers::META, "C", "Super+C"),
            ("ours meta", ours(), Modifiers::META, "c", "Super+C"),
            ("ours ctrl", ours(), Modifiers::CONTROL, "c", "Ctrl+C"),
            (
                "windows ctrl",
                Platform::Windows,
                Modifiers::CONTROL,
                "c",
                "Ctrl+C",
            ),
            (
                "windows meta",
                Platform::Windows,
                Modifiers::META,
                "c",
                "Super+C",
            ),
            (
                "kde meta shift",
                kde(),
                Modifiers::META | Modifiers::SHIFT,
                "s",
                "Shift+Super+S",
            ),
            (
                "locks are not modifiers",
                kde(),
                Modifiers::CONTROL | Modifiers::CAPS_LOCK | Modifiers::NUM_LOCK,
                "a",
                "Ctrl+A",
            ),
        ];
        for (name, platform, held, key, expected) in cases {
            let input = key_input(&Key::Character((*key).to_owned()), *held);
            let wanted: Chord = expected
                .parse()
                .unwrap_or_else(|e| panic!("{name}: {expected}: {e}"));
            let got = input.map(|input| input.normalise(*platform));
            assert_eq!(got, Some(wanted), "{name}");
        }
    }

    #[test]
    fn modifiers_drop_locks_and_alt_graph() {
        let held = Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::META | Modifiers::CAPS_LOCK;
        assert_eq!(
            modifiers_of(held | Modifiers::ALT_GRAPH),
            Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::META
        );
    }

    #[test]
    fn keys_map_to_chord_keys_and_the_rest_to_none() {
        let cases: Vec<(Key, Option<ChordKey>)> = vec![
            (Key::Character(" ".into()), Some(ChordKey::SPACE)),
            (Key::Character("Z".into()), Some(ChordKey::Char('z'))),
            (Key::Character("ab".into()), None),
            (
                Key::Named(NamedKey::Enter),
                Some(ChordKey::Named(NamedKey::Enter)),
            ),
            (
                Key::Named(NamedKey::ArrowLeft),
                Some(ChordKey::Named(NamedKey::ArrowLeft)),
            ),
            (Key::Named(NamedKey::F5), ChordKey::function(5)),
            (Key::Named(NamedKey::F24), ChordKey::function(24)),
            (
                Key::Named(NamedKey::Control),
                Some(ChordKey::Named(NamedKey::Control)),
            ),
        ];
        for (key, want) in cases {
            assert_eq!(chord_key_of(&key), want, "{key:?}");
        }
    }

    #[test]
    fn alt_graph_types_text_and_reads_no_chord() {
        let held = Modifiers::CONTROL | Modifiers::ALT | Modifiers::ALT_GRAPH;
        let input = key_input(&Key::Character("@".into()), held).expect("a chord key");
        assert!(input.types_text());
        let plain = key_input(
            &Key::Character("t".into()),
            Modifiers::CONTROL | Modifiers::ALT,
        )
        .expect("a chord key");
        assert!(!plain.types_text());
    }
}
