//! Changing keys of a TOML settings file's text, as text: every other byte, comment and the
//! person's formatting stay as they were. Reading and writing the file is
//! `settings_store_fs::SettingsWriter`.

use settings_types::schema::KeyPath;
use toml_edit::{DocumentMut, InlineTable, Item, Table, TableLike, Value};

/// One change to a settings file.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// Set the key (a dotted path) to the value, making the tables on the way.
    Set(KeyPath, toml::Value),
    /// Take the key out, its line and comments with it.
    Unset(KeyPath),
}

/// Why an edit could not be made to a file's text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    /// The text is not valid TOML.
    #[error("{0}")]
    NotToml(String),
    /// A key path with an empty part.
    #[error("{0:?} is not a key path")]
    NotAKeyPath(String),
    /// The key sits inside a value that is not a table.
    #[error("{0} sits inside a value that is not a table")]
    NotATable(String),
}

/// `text` with every edit applied, in order. If any edit fails nothing is returned.
///
/// # Errors
///
/// Returns an [`EditError`] when `text` is not valid TOML, a key path is empty in a part, or a key
/// sits inside a value that is not a table.
pub fn apply_edits(text: &str, edits: &[Edit]) -> Result<String, EditError> {
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|e: toml_edit::TomlError| EditError::NotToml(e.message().to_owned()))?;
    for edit in edits {
        let key = match edit {
            Edit::Set(key, _) | Edit::Unset(key) => key,
        };
        let parts: Vec<&str> = key.0.split('.').collect();
        if parts.iter().any(|part| part.is_empty()) {
            return Err(EditError::NotAKeyPath(key.0.clone()));
        }
        let done = match edit {
            Edit::Set(_, value) => set(doc.as_table_mut(), &parts, to_edit(value)),
            Edit::Unset(_) => unset(doc.as_table_mut(), &parts),
        };
        done.map_err(|()| EditError::NotATable(key.0.clone()))?;
    }
    Ok(doc.to_string())
}

/// `value` as a document value; a table becomes an inline table.
fn to_edit(value: &toml::Value) -> Value {
    match value {
        toml::Value::String(text) => Value::from(text.as_str()),
        toml::Value::Integer(number) => Value::from(*number),
        toml::Value::Float(number) => Value::from(*number),
        toml::Value::Boolean(flag) => Value::from(*flag),
        toml::Value::Datetime(when) => when
            .to_string()
            .parse()
            .unwrap_or_else(|_| Value::from(when.to_string())),
        toml::Value::Array(items) => Value::Array(items.iter().map(to_edit).collect()),
        toml::Value::Table(table) => {
            let mut inline = InlineTable::new();
            for (name, item) in table {
                inline.insert(name, to_edit(item));
            }
            Value::InlineTable(inline)
        }
    }
}

/// Sets `path` in `table` (a real or inline table), making the tables on the way. The key stays
/// where it is, with its own spacing and comments; only the value is replaced, taking the old
/// value's spacing and trailing comment.
fn set(table: &mut dyn TableLike, path: &[&str], value: Value) -> Result<(), ()> {
    match path {
        [] => Ok(()),
        [last] => {
            match table.get_mut(last) {
                Some(item) => {
                    let mut value = value;
                    if let Some(old) = item.as_value() {
                        *value.decor_mut() = old.decor().clone();
                    }
                    *item = Item::Value(value);
                }
                None => {
                    table.insert(last, Item::Value(value));
                }
            }
            Ok(())
        }
        [first, rest @ ..] => {
            if table.get(first).is_none() {
                let mut made = Table::new();
                made.set_implicit(true);
                table.insert(first, Item::Table(made));
            }
            let inner = table
                .get_mut(first)
                .and_then(Item::as_table_like_mut)
                .ok_or(())?;
            set(inner, rest, value)
        }
    }
}

/// Takes `path` out of `table`. A missing key or table is already gone; a value that is not a
/// table where the path goes on is refused.
fn unset(table: &mut dyn TableLike, path: &[&str]) -> Result<(), ()> {
    match path {
        [] => Ok(()),
        [last] => {
            table.remove(last);
            Ok(())
        }
        [first, rest @ ..] => match table.get_mut(first) {
            None => Ok(()),
            Some(item) => unset(item.as_table_like_mut().ok_or(())?, rest),
        },
    }
}
