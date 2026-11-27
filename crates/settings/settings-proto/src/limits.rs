//! How much a module may send.
//!
//! A module is another program: a reply that carries a million rows or a megabyte label would stall
//! a page that draws one widget per row, so a reader refuses what no real module sends.

use crate::LiveSchema;
use settings_types::schema::{KeyKind, KeySpec};

/// The longest schema text (`Describe`, `Snapshot`) read, in bytes. This also bounds the sum of
/// every string inside it.
pub const SCHEMA_JSON_MAX: usize = 1 << 20;
/// The most keys a schema, or a snapshot's values, may hold. An accounts page with a few dozen
/// accounts has a few hundred.
pub const KEYS_MAX: usize = 1024;
/// The longest key path, label, help line, note, section name or control name, in bytes.
pub const TEXT_MAX: usize = 2048;
/// The most choices one menu or segmented key may list.
pub const CHOICES_MAX: usize = 512;
/// The longest string value (`Get`, `Snapshot`, `Changed`), in bytes.
pub const VALUE_TEXT_MAX: usize = 8 * 1024;
/// The most items in a list value.
pub const VALUE_ITEMS_MAX: usize = 512;

/// Whether `count` rows fit a page.
#[must_use]
pub const fn rows_fit(count: usize) -> bool {
    count <= KEYS_MAX
}

/// Whether `schema` is within the limits: its key count, and every text, choice and default in it.
#[must_use]
pub fn schema_fits(schema: &LiveSchema) -> bool {
    rows_fit(schema.key.len()) && schema.key.iter().all(key_fits)
}

fn key_fits(spec: &KeySpec) -> bool {
    let choices: &[String] = match &spec.kind {
        KeyKind::Menu { variants } | KeyKind::Segmented { variants } => variants,
        _ => &[],
    };
    let texts = [
        spec.path.0.as_str(),
        spec.label.0.as_str(),
        spec.help.0.as_str(),
        spec.section.0.as_str(),
        spec.note.as_ref().map_or("", |note| note.0.as_str()),
        spec.control
            .as_ref()
            .map_or("", |control| control.0.as_str()),
    ];
    choices.len() <= CHOICES_MAX
        && texts.iter().all(|text| text.len() <= TEXT_MAX)
        && choices.iter().all(|word| word.len() <= TEXT_MAX)
        && value_fits(&spec.default)
}

/// Whether `value` is one a page can show: a string or list within the limits.
#[must_use]
pub fn value_fits(value: &toml::Value) -> bool {
    match value {
        toml::Value::String(text) => text.len() <= VALUE_TEXT_MAX,
        toml::Value::Array(items) => {
            items.len() <= VALUE_ITEMS_MAX
                && items.iter().all(|item| match item {
                    toml::Value::String(text) => text.len() <= VALUE_TEXT_MAX,
                    _ => true,
                })
        }
        _ => true,
    }
}
