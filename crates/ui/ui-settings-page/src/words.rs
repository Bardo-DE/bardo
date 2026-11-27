//! A stored word as a person reads it.

use settings_types::schema::KeySpec;

/// A stored word as a person reads it: the schema's label when it has one, else the word with
/// its first letter capitalised and underscores as spaces.
pub(crate) fn word_label(key: &KeySpec, word: &str) -> String {
    if let Some(label) = key.labels.of(word) {
        return label.to_owned();
    }
    let spaced = word.replace('_', " ");
    let mut chars = spaced.chars();
    chars
        .next()
        .map_or_default(|first| first.to_uppercase().chain(chars).collect())
}
