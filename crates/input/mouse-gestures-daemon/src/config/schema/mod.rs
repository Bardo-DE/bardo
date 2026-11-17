//! `mouse-gestures.settings.toml`: the schema of `gestures.toml` for the Settings app (Bardo
//! design/22 section 9).
//!
//! palmrest is a standalone daemon and takes no Bardo crate, so it cannot use Bardo's
//! `SettingsSchema` derive (and its own settings come from `settings_struct!`, which has no
//! serde derive to hang one on). This module writes the same file format from a table
//! (`keys.rs`) whose defaults are read from [`Config::default`], and the tests pin every row to
//! the config: each key of the config has a row, and each row's range and words are values the
//! lenient reader accepts.

mod keys;
#[cfg(test)]
mod tests;

use super::Config;
use serde::Serialize;
use std::collections::BTreeMap;

/// The file this schema describes, under `$XDG_CONFIG_HOME`.
const FILE: &str = "mouse-gestures/gestures.toml";
/// The schema file format's version (Bardo design/22 section 9.2).
const VERSION: u16 = 1;

/// A key's shape: what values it holds. Serialises as Bardo's `KeyKind`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub(super) enum Kind {
    /// Two words.
    Toggle { variants: [String; 2] },
    /// Three to five words.
    Segmented { variants: Vec<String> },
    /// More than five words.
    Menu { variants: Vec<String> },
    /// A number between two limits, in a unit.
    Bounded {
        min: i64,
        max: i64,
        unit: Option<String>,
    },
}

impl Kind {
    /// The kind of a closed set of `words`: Bardo's rule by count (two a toggle, three to five
    /// segments, more a menu).
    pub(super) fn choice(words: &[&str]) -> Kind {
        let owned: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
        match <[String; 2]>::try_from(owned.clone()) {
            Ok(variants) => Kind::Toggle { variants },
            Err(_) if owned.len() <= 5 => Kind::Segmented { variants: owned },
            Err(_) => Kind::Menu { variants: owned },
        }
    }
}

/// Whether the Settings app draws a key on its page or under its "Advanced" disclosure; every
/// palmrest key is on the page.
const EXPOSURE: &str = "basic";

/// The page every palmrest key is on, as Bardo's `Page` writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
struct Page {
    kind: &'static str,
}

const PAGE: Page = Page {
    kind: "mouse_and_gestures",
};

/// One key, as Bardo's `KeySpec` writes it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(super) struct KeySpec {
    pub path: String,
    pub default: toml::Value,
    pub label: String,
    pub help: String,
    pub section: String,
    pub exposure: &'static str,
    pub kind: Kind,
    page: Page,
    /// A human label per word of a choice, by stored word; omitted when the key has none.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,
}

/// The whole schema file.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Schema {
    app: String,
    file: String,
    version: u16,
    key: Vec<KeySpec>,
}

/// The name the file is written under in the schema directory.
pub const FILE_NAME: &str = "mouse-gestures.settings.toml";

impl Schema {
    /// palmrest's schema: one key per setting of [`Config`].
    ///
    /// # Panics
    /// never: every field of `Config` is a plain value or table.
    #[must_use]
    pub fn palmrest() -> Schema {
        let defaults = toml::Value::try_from(Config::default())
            .expect("every field of Config is a plain value or table");
        Schema {
            app: "mouse-gestures".to_owned(),
            file: FILE.to_owned(),
            version: VERSION,
            key: keys::rows(&defaults),
        }
    }

    /// The file's text.
    ///
    /// # Panics
    /// never: a schema is strings, integers and tables.
    #[must_use]
    pub fn to_toml(&self) -> String {
        toml::to_string(self).expect("a schema is strings, integers and tables")
    }
}
