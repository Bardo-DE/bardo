//! The `key` verb's spelling of a keystroke: `<name>[+mod[+mod]]`, where `name` is a W3C key
//! value (`Tab`, `Enter`, `Escape`, `ArrowDown`, `F5`, `a`, `Shift`) or `Space`, and a `mod` is
//! `shift`, `ctrl`, `alt` or `super`. `+` splits, so it cannot itself be a key name. A shifted
//! symbol (`@`, `!`, `_`, `:` …) is typed as its US key with Shift held, which the spec adds itself.

use std::str::FromStr;

use keyboard_types::{Code, Key, Modifiers, NamedKey};

use super::KeyStroke;
use super::command::CommandError;

/// The keystroke `spec` names (`Tab`, `k+ctrl`, `Tab+shift`): the `key` verb's spelling.
///
/// # Errors
///
/// `CommandError::NotAKey` when the key has no name or code, and `NotAModifier` for a modifier word
/// that is not one.
pub fn parse_key(spec: &str) -> Result<KeyStroke, CommandError> {
    let mut parts = spec.split('+');
    let name = parts.next().unwrap_or_default();
    let key = key_named(name).ok_or_else(|| CommandError::NotAKey(name.to_owned()))?;
    let code = code_of(&key).ok_or_else(|| CommandError::NotAKey(name.to_owned()))?;
    let mods = parts.try_fold(shift_of(&key), |held, word| {
        modifier(word)
            .map(|one| held | one)
            .ok_or_else(|| CommandError::NotAModifier(word.to_owned()))
    })?;
    Ok(KeyStroke { key, code, mods })
}

fn key_named(name: &str) -> Option<Key> {
    match name {
        "Space" => Some(Key::Character(" ".to_owned())),
        // The OS key is Meta; the deprecated Super name is read as it.
        "Super" | "Meta" => Some(Key::Named(NamedKey::Meta)),
        _ => Key::from_str(name).ok(),
    }
}

/// Shift for a symbol that a US layout types with Shift held (`@` is Shift+2), else nothing.
fn shift_of(key: &Key) -> Modifiers {
    match key {
        Key::Character(text) if text.chars().count() == 1 => text
            .chars()
            .next()
            .filter(|c| SHIFTED.iter().any(|(ch, _)| ch == c))
            .map_or(Modifiers::empty(), |_| Modifiers::SHIFT),
        _ => Modifiers::empty(),
    }
}

fn modifier(word: &str) -> Option<Modifiers> {
    match word {
        "shift" => Some(Modifiers::SHIFT),
        "ctrl" => Some(Modifiers::CONTROL),
        "alt" => Some(Modifiers::ALT),
        "super" => Some(Modifiers::META),
        _ => None,
    }
}

/// The physical key that types `key` on a US layout: a named key's code has its name, a
/// character is a letter, digit or punctuation key, a modifier is its left-hand key.
fn code_of(key: &Key) -> Option<Code> {
    match key {
        Key::Character(text) => text
            .chars()
            .next()
            .filter(|_| text.chars().count() == 1)
            .and_then(code_of_char),
        Key::Named(NamedKey::Shift) => Some(Code::ShiftLeft),
        Key::Named(NamedKey::Control) => Some(Code::ControlLeft),
        Key::Named(NamedKey::Alt) => Some(Code::AltLeft),
        Key::Named(NamedKey::Meta) => Some(Code::MetaLeft),
        named @ Key::Named(_) => Code::from_str(&named.to_string()).ok(),
    }
}

fn code_of_char(c: char) -> Option<Code> {
    let name = match c {
        'a'..='z' => format!("Key{}", c.to_ascii_uppercase()),
        'A'..='Z' => format!("Key{c}"),
        '0'..='9' => format!("Digit{c}"),
        ' ' => "Space".to_owned(),
        _ => {
            return PUNCTUATION
                .iter()
                .chain(SHIFTED)
                .find(|(ch, _)| *ch == c)
                .map(|(_, code)| *code);
        }
    };
    Code::from_str(&name).ok()
}

const PUNCTUATION: &[(char, Code)] = &[
    ('-', Code::Minus),
    ('=', Code::Equal),
    ('[', Code::BracketLeft),
    (']', Code::BracketRight),
    ('\\', Code::Backslash),
    (';', Code::Semicolon),
    ('\'', Code::Quote),
    ('`', Code::Backquote),
    (',', Code::Comma),
    ('.', Code::Period),
    ('/', Code::Slash),
];

/// The symbols a US layout types with Shift held, and the key under each (`+` is not here: it
/// splits a spec).
const SHIFTED: &[(char, Code)] = &[
    ('!', Code::Digit1),
    ('@', Code::Digit2),
    ('#', Code::Digit3),
    ('$', Code::Digit4),
    ('%', Code::Digit5),
    ('^', Code::Digit6),
    ('&', Code::Digit7),
    ('*', Code::Digit8),
    ('(', Code::Digit9),
    (')', Code::Digit0),
    ('_', Code::Minus),
    ('{', Code::BracketLeft),
    ('}', Code::BracketRight),
    ('|', Code::Backslash),
    (':', Code::Semicolon),
    ('"', Code::Quote),
    ('~', Code::Backquote),
    ('<', Code::Comma),
    ('>', Code::Period),
    ('?', Code::Slash),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke(key: Key, code: Code, mods: Modifiers) -> KeyStroke {
        KeyStroke { key, code, mods }
    }

    #[test]
    fn a_spec_names_key_code_and_modifiers() {
        let ch = |c: &str| Key::Character(c.to_owned());
        let none = Modifiers::empty();
        let cases: Vec<(&str, Result<KeyStroke, CommandError>)> = vec![
            (
                "Tab",
                Ok(stroke(Key::Named(NamedKey::Tab), Code::Tab, none)),
            ),
            (
                "Enter",
                Ok(stroke(Key::Named(NamedKey::Enter), Code::Enter, none)),
            ),
            (
                "ArrowDown",
                Ok(stroke(
                    Key::Named(NamedKey::ArrowDown),
                    Code::ArrowDown,
                    none,
                )),
            ),
            (
                "Home",
                Ok(stroke(Key::Named(NamedKey::Home), Code::Home, none)),
            ),
            ("F5", Ok(stroke(Key::Named(NamedKey::F5), Code::F5, none))),
            ("a", Ok(stroke(ch("a"), Code::KeyA, none))),
            ("7", Ok(stroke(ch("7"), Code::Digit7, none))),
            ("Space", Ok(stroke(ch(" "), Code::Space, none))),
            (",", Ok(stroke(ch(","), Code::Comma, none))),
            ("@", Ok(stroke(ch("@"), Code::Digit2, Modifiers::SHIFT))),
            ("_", Ok(stroke(ch("_"), Code::Minus, Modifiers::SHIFT))),
            (":", Ok(stroke(ch(":"), Code::Semicolon, Modifiers::SHIFT))),
            (
                "@+ctrl",
                Ok(stroke(
                    ch("@"),
                    Code::Digit2,
                    Modifiers::SHIFT | Modifiers::CONTROL,
                )),
            ),
            (
                "Shift",
                Ok(stroke(Key::Named(NamedKey::Shift), Code::ShiftLeft, none)),
            ),
            (
                "k+ctrl",
                Ok(stroke(ch("k"), Code::KeyK, Modifiers::CONTROL)),
            ),
            (
                "Super",
                Ok(stroke(Key::Named(NamedKey::Meta), Code::MetaLeft, none)),
            ),
            (
                "Tab+shift",
                Ok(stroke(
                    Key::Named(NamedKey::Tab),
                    Code::Tab,
                    Modifiers::SHIFT,
                )),
            ),
            (
                "y+super+alt",
                Ok(stroke(
                    ch("y"),
                    Code::KeyY,
                    Modifiers::META | Modifiers::ALT,
                )),
            ),
            ("Tabb", Err(CommandError::NotAKey("Tabb".into()))),
            ("", Err(CommandError::NotAKey(String::new()))),
            ("Tab+hyper", Err(CommandError::NotAModifier("hyper".into()))),
        ];
        for (spec, want) in cases {
            assert_eq!(parse_key(spec), want, "{spec:?}");
        }
    }
}
