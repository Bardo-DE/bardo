//! The catalogue directories, read.

use inference_router::catalog::{Catalog, CatalogDirs, Skipped, parse_entry_text};
use model_catalog::{ModelEntry, merge_catalogs};
use std::path::{Path, PathBuf};

/// Reads the system and user directories and merges them (a user file replaces the system file
/// of the same id). A missing directory is empty; a file that does not parse is skipped.
#[must_use]
pub fn read_catalog(dirs: &CatalogDirs) -> Catalog {
    let (system, mut skipped) = read_dir(&dirs.system);
    let (user, more) = read_dir(&dirs.user);
    skipped.extend(more);
    Catalog {
        entries: merge_catalogs(system, user),
        skipped,
    }
}

fn read_dir(dir: &Path) -> (Vec<ModelEntry>, Vec<Skipped>) {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    files.sort();
    let mut entries = Vec::new();
    let mut skipped = Vec::new();
    for path in files {
        let parsed = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|text| parse_entry_text(&text).map_err(|e| e.to_string()));
        match parsed {
            Ok(entry) => entries.push(entry),
            Err(why) => skipped.push(Skipped {
                what: path.display().to_string(),
                why,
            }),
        }
    }
    (entries, skipped)
}

#[cfg(test)]
mod tests;
