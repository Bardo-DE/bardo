//! Reading and writing chords as text: "Primary+Shift+Z", "ctrl+shift+z", "⇧⌘Z".

use std::str::FromStr;

use thiserror::Error;

use keyboard_types::{Modifiers, NamedKey, UnrecognizedNamedKeyError};

use crate::chord::{Chord, DefaultChord};
use crate::key::Key;
use crate::modifier::{key_of_modifier, modifier_of_key};
use crate::platform::{Desktop, Platform};

/// Why a chord's text could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ChordError {
    /// Nothing was written.
    #[error("the shortcut is empty")]
    Empty,
    /// A word that is neither a modifier nor a key.
    #[error("\"{0}\" is not a key I know")]
    UnknownKey(String),
    /// A word before the key that is not a modifier.
    #[error("\"{0}\" is not a modifier (Ctrl, Alt, Shift or Super)")]
    NotAModifier(String),
    /// Only modifiers were written.
    #[error("a shortcut needs a key as well as its modifiers")]
    NoKey,
    /// `Primary` where only a concrete chord is allowed.
    #[error(
        "\"Primary\" means Command or Ctrl depending on the platform, so it is not allowed here"
    )]
    PrimaryNotAllowed,
}

enum Word {
    Primary,
    Physical(Modifiers),
}

const GLYPHS: [(char, Modifiers); 4] = [
    ('⌃', Modifiers::CONTROL),
    ('⌥', Modifiers::ALT),
    ('⇧', Modifiers::SHIFT),
    ('⌘', Modifiers::META),
];

type KeyWord = (&'static str, Key);

const KEY_WORDS: &[KeyWord] = &[
    ("space", Key::SPACE),
    ("tab", Key::Named(NamedKey::Tab)),
    ("⇥", Key::Named(NamedKey::Tab)),
    ("enter", Key::Named(NamedKey::Enter)),
    ("return", Key::Named(NamedKey::Enter)),
    ("↩", Key::Named(NamedKey::Enter)),
    ("esc", Key::Named(NamedKey::Escape)),
    ("escape", Key::Named(NamedKey::Escape)),
    ("⎋", Key::Named(NamedKey::Escape)),
    ("backspace", Key::Named(NamedKey::Backspace)),
    ("⌫", Key::Named(NamedKey::Backspace)),
    ("delete", Key::Named(NamedKey::Delete)),
    ("del", Key::Named(NamedKey::Delete)),
    ("⌦", Key::Named(NamedKey::Delete)),
    ("insert", Key::Named(NamedKey::Insert)),
    ("ins", Key::Named(NamedKey::Insert)),
    ("home", Key::Named(NamedKey::Home)),
    ("↖", Key::Named(NamedKey::Home)),
    ("end", Key::Named(NamedKey::End)),
    ("↘", Key::Named(NamedKey::End)),
    ("pageup", Key::Named(NamedKey::PageUp)),
    ("pgup", Key::Named(NamedKey::PageUp)),
    ("⇞", Key::Named(NamedKey::PageUp)),
    ("pagedown", Key::Named(NamedKey::PageDown)),
    ("pgdn", Key::Named(NamedKey::PageDown)),
    ("⇟", Key::Named(NamedKey::PageDown)),
    ("up", Key::Named(NamedKey::ArrowUp)),
    ("↑", Key::Named(NamedKey::ArrowUp)),
    ("down", Key::Named(NamedKey::ArrowDown)),
    ("↓", Key::Named(NamedKey::ArrowDown)),
    ("left", Key::Named(NamedKey::ArrowLeft)),
    ("←", Key::Named(NamedKey::ArrowLeft)),
    ("right", Key::Named(NamedKey::ArrowRight)),
    ("→", Key::Named(NamedKey::ArrowRight)),
    ("printscreen", Key::Named(NamedKey::PrintScreen)),
    ("print", Key::Named(NamedKey::PrintScreen)),
    ("prtsc", Key::Named(NamedKey::PrintScreen)),
    ("plus", Key::Char('+')),
    ("minus", Key::Char('-')),
    ("comma", Key::Char(',')),
    ("period", Key::Char('.')),
    ("backtick", Key::Char('`')),
    ("grave", Key::Char('`')),
    ("slash", Key::Char('/')),
];

fn word(token: &str) -> Option<Word> {
    match token.to_lowercase().as_str() {
        "primary" => Some(Word::Primary),
        "ctrl" | "control" | "ctl" => Some(Word::Physical(Modifiers::CONTROL)),
        "alt" | "option" | "opt" => Some(Word::Physical(Modifiers::ALT)),
        "shift" => Some(Word::Physical(Modifiers::SHIFT)),
        "super" | "cmd" | "command" | "win" | "windows" | "meta" | "logo" => {
            Some(Word::Physical(Modifiers::META))
        }
        _ => None,
    }
}

fn parse_key(token: &str) -> Result<Key, ChordError> {
    let lower = token.to_lowercase();
    if let Some((_, key)) = KEY_WORDS.iter().find(|entry| entry.0 == lower) {
        return Ok(*key);
    }
    let function = lower
        .strip_prefix('f')
        .and_then(|digits| digits.parse::<u8>().ok())
        .and_then(Key::function);
    if let Some(key) = function {
        return Ok(key);
    }
    let mut chars = token.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Ok(Key::character(c)),
        _ => match token.parse::<NamedKey>() {
            Ok(named) => Ok(Key::Named(named)),
            // The parse error is a unit that says only "not a named key", as `UnknownKey` does.
            Err(UnrecognizedNamedKeyError) => Err(ChordError::UnknownKey(token.to_owned())),
        },
    }
}

/// The key of a lone modifier: that modifier tapped.
fn tap(modifier: Modifiers) -> Result<Key, ChordError> {
    key_of_modifier(modifier)
        .map(Key::Named)
        .ok_or(ChordError::NoKey)
}

fn take_glyphs(text: &str) -> (Modifiers, &str) {
    let mut modifiers = Modifiers::empty();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        match GLYPHS.iter().find(|(glyph, _)| *glyph == c) {
            Some((_, modifier)) => {
                modifiers = modifiers.union(*modifier);
                rest = &rest[c.len_utf8()..];
            }
            None => break,
        }
    }
    (modifiers, rest)
}

/// Splits "Ctrl+Shift+Z" into ("Ctrl+Shift", "Z"), keeping a trailing `+` key whole.
fn split_last(rest: &str) -> (&str, &str) {
    if rest == "+" {
        ("", "+")
    } else if let Some(head) = rest.strip_suffix("++") {
        (head, "+")
    } else {
        rest.rsplit_once('+').unwrap_or(("", rest))
    }
}

fn parse_default(text: &str) -> Result<DefaultChord, ChordError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(ChordError::Empty);
    }
    let (glyphs, rest) = take_glyphs(text);
    let mut modifiers = glyphs;
    let mut primary = false;
    let (head, last) = split_last(rest);
    for token in head.split('+').map(str::trim).filter(|t| !t.is_empty()) {
        match word(token) {
            Some(Word::Primary) => primary = true,
            Some(Word::Physical(modifier)) => modifiers = modifiers.union(modifier),
            None => return Err(ChordError::NotAModifier(token.to_owned())),
        }
    }
    let last = last.trim();
    let alone = modifiers.is_empty() && !primary;
    let key = if last.is_empty() {
        tap_of(glyphs, rest, alone)?
    } else {
        match word(last) {
            Some(Word::Physical(modifier)) if alone => tap(modifier)?,
            Some(_) => return Err(ChordError::NoKey),
            None => parse_key(last)?,
        }
    };
    let modifiers = if key.is_tap() {
        Modifiers::empty()
    } else {
        modifiers
    };
    Ok(DefaultChord::new(primary, Chord::new(modifiers, key)))
}

/// A bare glyph such as "⌘" is a tap of that modifier; anything else with no key is an error.
fn tap_of(glyphs: Modifiers, rest: &str, alone: bool) -> Result<Key, ChordError> {
    let mut held = glyphs.iter();
    match (held.next(), held.next()) {
        (Some(modifier), None) if rest.is_empty() => tap(modifier),
        _ if alone => Err(ChordError::Empty),
        _ => Err(ChordError::NoKey),
    }
}

impl FromStr for DefaultChord {
    type Err = ChordError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parse_default(text)
    }
}

impl FromStr for Chord {
    type Err = ChordError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let parsed = parse_default(text)?;
        if parsed.primary() {
            Err(ChordError::PrimaryNotAllowed)
        } else {
            Ok(parsed.base())
        }
    }
}

/// How a platform writes shortcuts.
enum Style {
    /// Symbols in the order ⌃⌥⇧⌘ with no separator.
    Glyphs,
    /// Words joined by `+`, modifiers in this order, the OS key under this name.
    Words {
        order: [Modifiers; 4],
        os_key: &'static str,
    },
}

const CTRL_FIRST: [Modifiers; 4] = [
    Modifiers::CONTROL,
    Modifiers::ALT,
    Modifiers::SHIFT,
    Modifiers::META,
];
const QT_ORDER: [Modifiers; 4] = [
    Modifiers::META,
    Modifiers::CONTROL,
    Modifiers::ALT,
    Modifiers::SHIFT,
];

fn style_of(platform: Platform) -> Style {
    match platform {
        Platform::MacOs
        | Platform::Linux {
            desktop: Desktop::Ours,
        } => Style::Glyphs,
        Platform::Windows => Style::Words {
            order: CTRL_FIRST,
            os_key: "Win",
        },
        Platform::Linux {
            desktop: Desktop::Kde,
        } => Style::Words {
            order: QT_ORDER,
            os_key: "Meta",
        },
        Platform::Linux { .. } => Style::Words {
            order: CTRL_FIRST,
            os_key: "Super",
        },
    }
}

/// The glyph of one chord modifier.
fn glyph_of(modifier: Modifiers) -> &'static str {
    match modifier {
        m if m == Modifiers::CONTROL => "⌃",
        m if m == Modifiers::ALT => "⌥",
        m if m == Modifiers::SHIFT => "⇧",
        _ => "⌘",
    }
}

/// The word of one chord modifier, the OS key under `os_key`.
fn word_of(modifier: Modifiers, os_key: &'static str) -> &'static str {
    match modifier {
        m if m == Modifiers::CONTROL => "Ctrl",
        m if m == Modifiers::ALT => "Alt",
        m if m == Modifiers::SHIFT => "Shift",
        _ => os_key,
    }
}

fn key_text(key: Key, style: &Style) -> String {
    let glyphs = matches!(style, Style::Glyphs);
    let os_key: &'static str = match style {
        Style::Words { os_key, .. } => os_key,
        Style::Glyphs => "",
    };
    if let Some(n) = key.function_number() {
        return format!("F{n}");
    }
    match key {
        Key::Char(' ') => "Space".to_owned(),
        Key::Char(c) => c.to_uppercase().collect(),
        Key::Named(named) => match modifier_of_key(named) {
            Some(modifier) if glyphs => glyph_of(modifier).to_owned(),
            Some(modifier) => word_of(modifier, os_key).to_owned(),
            None => named_text(named, glyphs).map_or_else(|| named.to_string(), str::to_owned),
        },
    }
}

/// How a menu writes a named key, as a glyph and in words; `None` for a key it has no short
/// name for, which is then written by its keyboard-types name.
fn named_text(key: NamedKey, glyphs: bool) -> Option<&'static str> {
    let (glyph, words) = match key {
        NamedKey::Tab => ("⇥", "Tab"),
        NamedKey::Enter => ("↩", "Enter"),
        NamedKey::Escape => ("⎋", "Esc"),
        NamedKey::Backspace => ("⌫", "Backspace"),
        NamedKey::Delete => ("⌦", "Delete"),
        NamedKey::Insert => ("Ins", "Ins"),
        NamedKey::Home => ("↖", "Home"),
        NamedKey::End => ("↘", "End"),
        NamedKey::PageUp => ("⇞", "PgUp"),
        NamedKey::PageDown => ("⇟", "PgDn"),
        NamedKey::ArrowUp => ("↑", "Up"),
        NamedKey::ArrowDown => ("↓", "Down"),
        NamedKey::ArrowLeft => ("←", "Left"),
        NamedKey::ArrowRight => ("→", "Right"),
        NamedKey::PrintScreen => ("PrtSc", "PrtSc"),
        _ => return None,
    };
    Some(if glyphs { glyph } else { words })
}

fn render(prefix: &[&str], modifiers: Modifiers, key: Key, style: &Style) -> String {
    match style {
        Style::Glyphs => {
            let mods: String = GLYPHS
                .iter()
                .filter(|(_, modifier)| modifiers.contains(*modifier))
                .map(|(glyph, _)| *glyph)
                .collect();
            format!("{mods}{}", key_text(key, style))
        }
        Style::Words { order, os_key } => {
            let mods = order
                .iter()
                .filter(|modifier| modifiers.contains(**modifier))
                .map(|modifier| word_of(*modifier, os_key));
            let parts: Vec<String> = prefix
                .iter()
                .copied()
                .chain(mods)
                .map(str::to_owned)
                .chain([key_text(key, style)])
                .collect();
            parts.join("+")
        }
    }
}

impl Chord {
    /// The chord as `platform` shows it to a person: "⇧⌘Z" on a Mac or our desktop,
    /// "Ctrl+Shift+Z" elsewhere in that platform's order and words. `FromStr` reads it back.
    #[must_use]
    pub fn display(self, platform: Platform) -> String {
        render(&[], self.modifiers(), self.key(), &style_of(platform))
    }

    /// The platform-neutral text form (always words, fixed order), stable for files.
    #[must_use]
    pub fn canonical(self) -> String {
        DefaultChord::from(self).canonical()
    }
}

impl DefaultChord {
    /// The platform-neutral text form: "Primary+Shift+Z", always words, fixed order.
    #[must_use]
    pub fn canonical(self) -> String {
        let neutral = Style::Words {
            order: CTRL_FIRST,
            os_key: "Super",
        };
        let prefix: &[&str] = if self.primary() { &["Primary"] } else { &[] };
        render(prefix, self.base().modifiers(), self.base().key(), &neutral)
    }
}

#[cfg(test)]
mod tests;
