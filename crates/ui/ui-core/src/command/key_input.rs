//! A key press as chordkit reads it: the one place a toolkit's modifiers and key become a
//! [`keymap::KeyInput`]. Everything that resolves a key to an action goes through it, so no
//! other file decides what Ctrl, Super or Meta mean.

use keyboard_types::{Key, Modifiers};
use keymap::keyboard_types::{Key as LogicalKey, Modifiers as ChordModifiers, NamedKey};
use keymap::{Key as ChordKey, KeyInput};

// The toolkit (dioxus) hands in keyboard-types 0.7 values and chordkit reads 0.8: the two
// conversions below go away when dioxus moves to 0.8 (phase 1b).

/// The chordkit flags `held` reports, by name: Super and Meta are both the OS key (`META`).
/// Locks and the like are left out; `AltGraph` is kept for [`key_input`].
fn flags_of(held: Modifiers) -> ChordModifiers {
    [
        (Modifiers::CONTROL, ChordModifiers::CONTROL),
        (Modifiers::ALT, ChordModifiers::ALT),
        (Modifiers::SHIFT, ChordModifiers::SHIFT),
        (Modifiers::SUPER, ChordModifiers::META),
        (Modifiers::META, ChordModifiers::META),
        (Modifiers::ALT_GRAPH, ChordModifiers::ALT_GRAPH),
    ]
    .into_iter()
    .filter(|(flag, _)| held.contains(*flag))
    .fold(ChordModifiers::empty(), |flags, (_, mapped)| flags | mapped)
}

/// The chord modifiers `held` reports (Ctrl, Alt, Shift and the OS key). Lock and `AltGraph`
/// flags are not chord modifiers.
#[must_use]
pub fn modifiers_of(held: Modifiers) -> ChordModifiers {
    flags_of(held).difference(ChordModifiers::ALT_GRAPH)
}

/// The toolkit flags that stand for the chord modifiers `held`: the reverse of
/// [`modifiers_of`], for a test or a driver that presses a chord. The OS key is `META`.
#[must_use]
pub fn modifiers_held(held: ChordModifiers) -> Modifiers {
    [
        (ChordModifiers::CONTROL, Modifiers::CONTROL),
        (ChordModifiers::ALT, Modifiers::ALT),
        (ChordModifiers::SHIFT, Modifiers::SHIFT),
        (ChordModifiers::META, Modifiers::META),
    ]
    .into_iter()
    .filter(|(flag, _)| held.contains(*flag))
    .fold(Modifiers::empty(), |flags, (_, mapped)| flags | mapped)
}

/// `key` as chordkit's keyboard-types names it, for the keys a chord can have; `None` for a
/// modifier on its own, a media key or any other.
fn logical_key(key: &Key) -> Option<LogicalKey> {
    let named = LogicalKey::Named;
    Some(match key {
        Key::Character(text) => LogicalKey::Character(text.clone()),
        Key::Enter => named(NamedKey::Enter),
        Key::Tab => named(NamedKey::Tab),
        Key::Escape => named(NamedKey::Escape),
        Key::Backspace => named(NamedKey::Backspace),
        Key::Delete => named(NamedKey::Delete),
        Key::Insert => named(NamedKey::Insert),
        Key::Home => named(NamedKey::Home),
        Key::End => named(NamedKey::End),
        Key::PageUp => named(NamedKey::PageUp),
        Key::PageDown => named(NamedKey::PageDown),
        Key::ArrowUp => named(NamedKey::ArrowUp),
        Key::ArrowDown => named(NamedKey::ArrowDown),
        Key::ArrowLeft => named(NamedKey::ArrowLeft),
        Key::ArrowRight => named(NamedKey::ArrowRight),
        Key::PrintScreen => named(NamedKey::PrintScreen),
        other => function_key(other)?,
    })
}

/// `F1` to `F24`, read from the key's name (`F5`), so no arm per function key.
fn function_key(key: &Key) -> Option<LogicalKey> {
    let name = key.to_string();
    let digits = name.strip_prefix('F')?;
    match ChordKey::function(digits.parse().ok()?)? {
        ChordKey::Named(named) => Some(LogicalKey::Named(named)),
        ChordKey::Char(_) => None,
    }
}

/// The chord key `key` is, or `None` for a key that is no chord key (a modifier on its own, a
/// multi-character string from an IME, a media key).
#[must_use]
pub fn chord_key_of(key: &Key) -> Option<ChordKey> {
    ChordKey::from_logical(&logical_key(key)?)
}

/// The key press `key` with `modifiers` held, as chordkit takes it.
///
/// The key is as the toolkit reports it: a shifted symbol (`!` for Shift+1) is not unshifted here,
/// so an app chord that holds Shift with a symbol key reads the base key from the physical code.
/// `AltGraph` rides in the flags: it types a character, and chordkit then reads no chord from the
/// press.
#[must_use]
pub fn key_input(key: &Key, modifiers: Modifiers) -> Option<KeyInput> {
    KeyInput::from_logical(flags_of(modifiers), &logical_key(key)?)
}

#[cfg(test)]
mod tests {
    use super::*;
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
                Modifiers::SUPER,
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
            ("ours super", ours(), Modifiers::SUPER, "C", "Super+C"),
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
    fn flags_and_modifiers_round_trip() {
        let held = Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::META;
        assert_eq!(modifiers_held(modifiers_of(held)), held);
        assert_eq!(
            modifiers_of(Modifiers::SUPER),
            modifiers_of(Modifiers::META)
        );
        assert_eq!(modifiers_held(ChordModifiers::empty()), Modifiers::empty());
    }

    #[test]
    fn keys_map_to_chord_keys_and_the_rest_to_none() {
        let cases: Vec<(Key, Option<ChordKey>)> = vec![
            (Key::Character(" ".into()), Some(ChordKey::SPACE)),
            (Key::Character("Z".into()), Some(ChordKey::Char('z'))),
            (Key::Character("ab".into()), None),
            (Key::Enter, Some(ChordKey::Named(NamedKey::Enter))),
            (Key::ArrowLeft, Some(ChordKey::Named(NamedKey::ArrowLeft))),
            (Key::F5, ChordKey::function(5)),
            (Key::F24, ChordKey::function(24)),
            (Key::Control, None),
            (Key::Fn, None),
            (Key::AudioVolumeUp, None),
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
