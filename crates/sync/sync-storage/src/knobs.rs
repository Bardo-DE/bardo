//! The `[storage]` table of syncd's own settings file.
//!
//! It is `$XDG_CONFIG_HOME/bardo/sync/syncd.toml`: the cache limit every drive evicts to, which drives are paused and which are kept downloaded
//! whole. Every write edits that table and nothing else, under the file's lock, so another
//! writer's key is never put back old.

use drive_fs::CacheLimit;
use std::collections::BTreeSet;
use std::path::Path;
use toml_edit::{Array, DocumentMut, Item};

/// The text of the settings file `file`, when there is one.
fn settings_text(file: &Path) -> Option<String> {
    std::fs::read_to_string(file).ok()
}

/// The cache limit `file` keeps (`[storage] cache_limit = "<bytes>"`); automatic when there is
/// no file, or it says nothing this build reads.
pub fn read_cache_limit(file: &Path) -> CacheLimit {
    let Some(text) = settings_text(file) else {
        return CacheLimit::Automatic;
    };
    let Ok(table) = text.parse::<toml::Table>() else {
        return CacheLimit::Automatic;
    };
    table
        .get("storage")
        .and_then(|storage| storage.get("cache_limit"))
        .and_then(bytes_of)
        .map_or(CacheLimit::Automatic, CacheLimit::from_setting)
}

/// The drives `file` keeps paused (`[storage] paused = ["<drive id>", ...]`).
#[must_use]
pub fn read_paused(file: &Path) -> BTreeSet<String> {
    read_drives(file, "paused")
}

/// Writes to `file` that the drive `id` is paused, or not, keeping the rest of it.
///
/// # Errors
///
/// Returns the I/O error when the file cannot be read, parsed or written.
pub fn write_paused(file: &Path, id: &str, paused: bool) -> std::io::Result<()> {
    write_drives(file, "paused", id, paused)
}

/// The drives `file` keeps downloaded whole (`[storage] keep_all = ["<drive id>", ...]`).
#[must_use]
pub fn read_keep_all(file: &Path) -> BTreeSet<String> {
    read_drives(file, "keep_all")
}

/// Writes to `file` that the drive `id` is kept downloaded whole, or not, keeping the rest of
/// it.
///
/// # Errors
///
/// Returns the I/O error when the file cannot be read, parsed or written.
pub fn write_keep_all(file: &Path, id: &str, on: bool) -> std::io::Result<()> {
    write_drives(file, "keep_all", id, on)
}

/// The drive ids `file` lists under `[storage] <list>`.
fn read_drives(file: &Path, list: &str) -> BTreeSet<String> {
    settings_text(file)
        .and_then(|text| text.parse::<toml::Table>().ok())
        .and_then(|table| {
            let ids = table.get("storage")?.get(list)?.as_array()?.clone();
            Some(
                ids.iter()
                    .filter_map(|id| id.as_str().map(str::to_owned))
                    .collect(),
            )
        })
        .unwrap_or_default()
}

/// Writes to `file` that the drive `id` is in `[storage] <list>`, or not, keeping the rest of it.
fn write_drives(file: &Path, list: &str, id: &str, listed: bool) -> std::io::Result<()> {
    edit_storage(file, |storage| {
        let mut ids: BTreeSet<String> =
            storage
                .get(list)
                .and_then(Item::as_array)
                .map_or_default(|ids| {
                    ids.iter()
                        .filter_map(|id| id.as_str().map(str::to_owned))
                        .collect()
                });
        if listed {
            ids.insert(id.to_owned())
        } else {
            ids.remove(id)
        };
        storage.insert(list, toml_edit::value(ids.into_iter().collect::<Array>()));
    })
}

/// Edits the `[storage]` table of `file` and nothing else: the read, the edit and the rename
/// happen under the file's lock (`base_fs::update_atomic`, the one every writer of the shared
/// settings file takes), so another writer's key is never put back old, and every other
/// section, key and comment keeps the text it had. A file that exists and does not parse as
/// TOML (or whose `storage` is not a table) is left untouched and the error says so.
fn edit_storage(
    file: &Path,
    edit: impl FnOnce(&mut dyn toml_edit::TableLike),
) -> std::io::Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    base_fs::update_atomic(
        file,
        base_fs::ReadCap::mib(1),
        base_fs::NewFileMode::OwnerOnly,
        |text| -> Result<String, std::io::Error> {
            let mut doc: DocumentMut = text.unwrap_or_default().parse().map_err(|why| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{} is not valid TOML, left as it is: {why}", file.display()),
                )
            })?;
            let storage = doc
                .as_table_mut()
                .entry("storage")
                .or_insert_with(|| {
                    let mut made = toml_edit::Table::new();
                    made.set_implicit(true);
                    Item::Table(made)
                })
                .as_table_like_mut()
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "{}: `storage` is not a table, left as it is",
                            file.display()
                        ),
                    )
                })?;
            edit(storage);
            Ok(doc.to_string())
        },
    )
    .map_err(|why| match why {
        base_fs::UpdateError::Edit(why) => why,
        other => std::io::Error::other(other),
    })
}

/// Writes `limit` to `file`, keeping the rest of it.
///
/// # Errors
///
/// Returns the I/O error when the file cannot be read, parsed or written.
pub fn write_cache_limit(file: &Path, limit: CacheLimit) -> std::io::Result<()> {
    edit_storage(file, |storage| {
        storage.insert("cache_limit", toml_edit::value(limit.setting().to_string()));
    })
}

/// Bytes as the row takes them: text of digits (or a whole number).
#[must_use]
pub fn bytes_of(value: &toml::Value) -> Option<u64> {
    match value {
        toml::Value::String(text) => text.trim().parse().ok(),
        toml::Value::Integer(n) => u64::try_from(*n).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn which_drives_are_kept_whole_is_saved_beside_the_rest_and_read_back() {
        let dir = base_testkit::Scratch::new("syncd-settings-keep-all");
        let file = dir.join("bardo/sync/syncd.toml");
        write_paused(&file, "one", true).expect("paused");
        write_keep_all(&file, "one", true).expect("save");
        write_keep_all(&file, "two", true).expect("save");
        write_keep_all(&file, "two", false).expect("save");
        assert_eq!(read_keep_all(&file), BTreeSet::from(["one".to_owned()]));
        assert_eq!(
            read_paused(&file),
            BTreeSet::from(["one".to_owned()]),
            "kept"
        );
    }

    #[test]
    fn a_file_that_does_not_parse_is_left_as_it_is_and_the_save_says_so() {
        let dir = base_testkit::Scratch::new("syncd-settings-garbled");
        let file = dir.join("syncd.toml");
        let garbled = "[appearance\ntheme = \"dark\"\n";
        std::fs::write(&file, garbled).expect("seed");
        assert!(write_paused(&file, "one", true).is_err());
        assert!(write_keep_all(&file, "one", true).is_err());
        assert!(write_cache_limit(&file, CacheLimit::Automatic).is_err());
        assert_eq!(std::fs::read_to_string(&file).expect("read"), garbled);
    }

    #[test]
    fn a_save_keeps_the_other_sections_and_the_comments() {
        let dir = base_testkit::Scratch::new("syncd-settings-keeps-rest");
        let file = dir.join("syncd.toml");
        let seeded = "# my settings\n[appearance]\ntheme = \"dark\" # night\n\n[storage]\n# drives\npaused = [\"a\"]\n";
        std::fs::write(&file, seeded).expect("seed");
        write_keep_all(&file, "b", true).expect("save");
        let text = std::fs::read_to_string(&file).expect("read");
        assert!(text.contains("# my settings"), "{text}");
        assert!(text.contains("theme = \"dark\" # night"), "{text}");
        assert!(text.contains("# drives"), "{text}");
        assert_eq!(read_paused(&file), BTreeSet::from(["a".to_owned()]));
        assert_eq!(read_keep_all(&file), BTreeSet::from(["b".to_owned()]));
    }

    #[test]
    fn saves_of_different_keys_from_many_threads_all_land() {
        let dir = base_testkit::Scratch::new("syncd-settings-interleaved");
        let file = dir.join("syncd.toml");
        let ids: Vec<String> = (0..8).map(|n| format!("drive{n}")).collect();
        std::thread::scope(|scope| {
            for id in &ids {
                let file = &file;
                scope.spawn(move || {
                    write_keep_all(file, id, true).expect("keep all");
                    write_paused(file, id, true).expect("paused");
                });
            }
            scope.spawn(|| {
                write_cache_limit(&file, CacheLimit::from_setting(5)).expect("limit");
            });
        });
        let all: BTreeSet<String> = ids.into_iter().collect();
        assert_eq!(read_keep_all(&file), all);
        assert_eq!(read_paused(&file), all);
        assert_eq!(read_cache_limit(&file), CacheLimit::from_setting(5));
    }
}
