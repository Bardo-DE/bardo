//! The remembered folders, as data and as text.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use files_core::{FolderPath, FolderSettings, Sort, SortKey, SortOrder, ViewMode};

/// How many folders are remembered; the one least recently changed is forgotten first.
pub const KEPT: usize = 500;

/// A folder's prefs and when they were set, as a count of changes.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Remembered {
    prefs: FolderSettings,
    set: u64,
}

/// Every folder whose view or sort differs from the defaults, newest changes kept up to [`KEPT`].
///
/// A folder set back to the defaults is forgotten, so the book holds only choices.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PrefBook {
    folders: BTreeMap<FolderPath, Remembered>,
    changes: u64,
}

impl PrefBook {
    /// How `folder` is remembered to be shown, if it is.
    #[must_use]
    pub fn get(&self, folder: &FolderPath) -> Option<FolderSettings> {
        self.folders.get(folder).map(|found| found.prefs)
    }

    /// Remembers `prefs` for `folder`, forgetting the least recently changed folder when the book
    /// is over [`KEPT`].
    pub fn set(&mut self, folder: FolderPath, prefs: FolderSettings) {
        if prefs == FolderSettings::default() {
            self.folders.remove(&folder);
            return;
        }
        self.changes += 1;
        let set = self.changes;
        self.folders.insert(folder, Remembered { prefs, set });
        while self.folders.len() > KEPT {
            let oldest = self
                .folders
                .iter()
                .min_by_key(|(_, found)| found.set)
                .map(|(path, _)| path.clone());
            match oldest {
                Some(path) => self.folders.remove(&path),
                None => break,
            };
        }
    }

    /// How many folders are remembered.
    #[must_use]
    pub fn len(&self) -> usize {
        self.folders.len()
    }

    /// Whether no folder is remembered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.folders.is_empty()
    }

    /// The book as stored text: one `[[folder]]` table per folder, oldest change first. A path
    /// that is not valid text is not stored.
    #[must_use]
    pub fn render(&self) -> String {
        let mut ordered: Vec<(&FolderPath, &Remembered)> = self.folders.iter().collect();
        ordered.sort_by_key(|(_, found)| found.set);
        let mut text = String::from("# How each folder is shown. Written by Files.\n");
        for (path, found) in ordered {
            let Some(path) = path.as_path().to_str() else {
                continue;
            };
            let quoted = |word: &str| toml::Value::String(word.to_owned()).to_string();
            let prefs = found.prefs;
            // Writing to a `String` cannot fail.
            let _ = write!(
                text,
                "\n[[folder]]\npath = {}\nview = {}\nkey = {}\norder = {}\n",
                quoted(path),
                quoted(prefs.view.slug()),
                quoted(prefs.sort.key.slug()),
                quoted(prefs.sort.order.slug()),
            );
        }
        text
    }

    /// The book `text` holds. A table with no usable path is skipped; a word this version does not
    /// know leaves that one setting at its default. `None` when `text` is not valid TOML.
    pub fn parse(text: &str) -> Option<PrefBook> {
        let table: toml::Table = text.parse().ok()?;
        let tables = table
            .get("folder")
            .and_then(toml::Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let mut book = PrefBook::default();
        for entry in tables.iter().filter_map(toml::Value::as_table) {
            let path = entry
                .get("path")
                .and_then(toml::Value::as_str)
                .and_then(|path| FolderPath::new(path).ok());
            if let Some(path) = path {
                book.set(path, prefs_of(entry));
            }
        }
        Some(book)
    }
}

/// The prefs one stored table holds.
fn prefs_of(entry: &toml::Table) -> FolderSettings {
    let word = |key: &str| entry.get(key).and_then(toml::Value::as_str);
    FolderSettings {
        view: word("view")
            .and_then(ViewMode::from_slug)
            .unwrap_or_default(),
        sort: Sort {
            key: word("key").and_then(SortKey::from_slug).unwrap_or_default(),
            order: word("order")
                .and_then(SortOrder::from_slug)
                .unwrap_or_default(),
        },
    }
}
