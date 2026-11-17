//! The reader of a daemon's own settings file, driven by a table of keys.
//!
//! Each key says what it takes (a whole number in a range, a switch, one of some words) and where its value lands in the
//! typed settings. A bad value costs only its own key, which keeps the `base` value and is
//! reported in [`Loaded::invalid`]; a key no row names is reported in [`Loaded::unknown`].
//!
//! This is the lenient read of a file whose keys are a flat table of rules rather than a serde
//! document (`agent.*`, `memory.*`, `computer_use.*`): the same [`Loaded`] comes out as for any other
//! settings file.

use crate::lenient::{InvalidKey, Loaded, UnknownKey};
use std::collections::BTreeSet;
use std::ops::RangeInclusive;

/// What a key's value may be and how it lands in the settings `T`.
#[derive(Debug)]
pub enum Rule<T> {
    /// A whole number inside `range`, in the key's unit.
    Number {
        /// The numbers the key takes.
        range: RangeInclusive<i64>,
        /// Puts the number in the settings.
        set: fn(&mut T, i64),
    },
    /// A switch, stored as TOML `true` or `false`.
    Toggle {
        /// Puts the switch in the settings.
        set: fn(&mut T, bool),
    },
    /// One of `words`: `set` reads the word and says whether the field took it.
    Word {
        /// The words the key takes, for the report of a refused value.
        words: &'static [&'static str],
        /// Puts the word in the settings; `false` when it is not one the field takes.
        set: fn(&mut T, &str) -> bool,
    },
}

/// One key of the table.
#[derive(Debug)]
pub struct RuleKey<T> {
    /// The dotted key path (`agent.budget.spend_microusd`).
    pub path: &'static str,
    /// What the key takes.
    pub rule: Rule<T>,
}

/// A number key: `path` takes a whole number in `range`.
pub const fn number<T>(
    path: &'static str,
    range: RangeInclusive<i64>,
    set: fn(&mut T, i64),
) -> RuleKey<T> {
    RuleKey {
        path,
        rule: Rule::Number { range, set },
    }
}

/// A switch key.
pub const fn toggle<T>(path: &'static str, set: fn(&mut T, bool)) -> RuleKey<T> {
    RuleKey {
        path,
        rule: Rule::Toggle { set },
    }
}

/// A word key: `path` takes one of `words`.
pub const fn word<T>(
    path: &'static str,
    words: &'static [&'static str],
    set: fn(&mut T, &str) -> bool,
) -> RuleKey<T> {
    RuleKey {
        path,
        rule: Rule::Word { words, set },
    }
}

/// The keys of a file, and what else it may hold without being reported.
#[derive(Debug)]
pub struct RuleTable<'a, T> {
    /// The scalar keys, read by their rules.
    pub keys: &'a [RuleKey<T>],
    /// Keys read some other way (a list the caller reads in `more`), and `version`: not unknown.
    pub also_known: &'a [&'a str],
    /// Paths under these prefixes belong to something else and are never reported.
    pub ignored_prefixes: &'a [&'a str],
}

/// The value at the dotted `path` of `table`.
#[must_use]
pub fn find<'a>(table: &'a toml::Table, path: &str) -> Option<&'a toml::Value> {
    let mut parts = path.split('.');
    let mut value = table.get(parts.next()?)?;
    for part in parts {
        value = value.as_table()?.get(part)?;
    }
    Some(value)
}

fn leaves(prefix: &str, table: &toml::Table, out: &mut Vec<String>) {
    for (name, value) in table {
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}.{name}")
        };
        match value {
            toml::Value::Table(inner) => leaves(&path, inner, out),
            _ => out.push(path),
        }
    }
}

fn refused<T>(rule: &Rule<T>, value: &mut T, found: &toml::Value) -> Option<String> {
    match rule {
        Rule::Number { range, set } => match found.as_integer() {
            None => Some("not a whole number".to_owned()),
            Some(n) if range.contains(&n) => {
                set(value, n);
                None
            }
            Some(_) => Some(format!("outside {}..={}", range.start(), range.end())),
        },
        Rule::Toggle { set } => match found.as_bool() {
            Some(on) => {
                set(value, on);
                None
            }
            None => Some("not true or false".to_owned()),
        },
        Rule::Word { words, set } => match found.as_str() {
            Some(word) if set(value, word) => None,
            _ => Some(format!("not one of {}", words.join(", "))),
        },
    }
}

/// Reads `text` over `base`, one key at a time.
///
/// Each key the file sets and the table accepts replaces its base
/// value; every other key keeps it. `more` reads what the table cannot (a list of rows) from the
/// parsed file, changing the value and reporting what it refused.
///
/// A text that is not TOML is `base` with the whole file reported invalid.
pub fn read_rules<T>(
    text: &str,
    mut value: T,
    table: &RuleTable<'_, T>,
    more: impl FnOnce(&toml::Table, &mut T, &mut Vec<InvalidKey>),
) -> Loaded<T> {
    let doc = match text.parse::<toml::Table>() {
        Ok(doc) => doc,
        Err(error) => return Loaded::garbled(value, format!("not TOML: {}", error.message())),
    };
    let mut invalid = Vec::new();
    for key in table.keys {
        let Some(found) = find(&doc, key.path) else {
            continue;
        };
        if let Some(reason) = refused(&key.rule, &mut value, found) {
            invalid.push(InvalidKey {
                path: key.path.to_owned(),
                reason,
            });
        }
    }
    more(&doc, &mut value, &mut invalid);
    let known: BTreeSet<&str> = table
        .keys
        .iter()
        .map(|key| key.path)
        .chain(table.also_known.iter().copied())
        .chain(["version"])
        .collect();
    let mut seen = Vec::new();
    leaves("", &doc, &mut seen);
    let unknown = seen
        .into_iter()
        .filter(|path| {
            !known.contains(path.as_str())
                && !table
                    .ignored_prefixes
                    .iter()
                    .any(|prefix| path.starts_with(prefix))
        })
        .map(UnknownKey)
        .collect();
    Loaded {
        value,
        unknown,
        invalid,
        clamped: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{RuleTable, number, read_rules, toggle, word};

    #[derive(Debug, Clone, PartialEq, Default)]
    struct Mine {
        size: i64,
        on: bool,
        shape: String,
    }

    fn keys() -> Vec<super::RuleKey<Mine>> {
        vec![
            number("a.size", 1..=10, |m, v| m.size = v),
            toggle("a.on", |m, v| m.on = v),
            word("a.shape", &["round", "square"], |m, w| {
                m.shape = w.to_owned();
                true
            }),
        ]
    }

    fn table(keys: &[super::RuleKey<Mine>]) -> RuleTable<'_, Mine> {
        RuleTable {
            keys,
            also_known: &[],
            ignored_prefixes: &["other."],
        }
    }

    #[test]
    fn a_bad_value_costs_its_own_key_and_an_unknown_key_is_reported() {
        let keys = keys();
        let loaded = read_rules(
            "version = 1\n[a]\nsize = 99\non = true\nbogus = 1\n[other]\nx = 1\n",
            Mine::default(),
            &table(&keys),
            |_, _, _| {},
        );
        assert_eq!(loaded.value.size, 0);
        assert!(loaded.value.on);
        assert_eq!(loaded.invalid.len(), 1);
        assert_eq!(loaded.invalid[0].path, "a.size");
        assert_eq!(loaded.invalid[0].reason, "outside 1..=10");
        assert_eq!(loaded.unknown.len(), 1);
        assert_eq!(loaded.unknown[0].0, "a.bogus");
    }

    #[test]
    fn text_that_is_not_toml_keeps_the_base() {
        let keys = keys();
        let loaded = read_rules("[a\n", Mine::default(), &table(&keys), |_, _, _| {});
        assert_eq!(loaded.value, Mine::default());
        assert_eq!(loaded.invalid.len(), 1);
        assert!(loaded.invalid[0].reason.starts_with("not TOML: "));
    }
}
