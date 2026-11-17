//! The include guard of a crate's one integration test executable (`tests/it/main.rs`): a file
//! nobody listed there compiles to nothing and its tests silently never run.

use std::path::{Path, PathBuf};

/// Fails when a `*.rs` file in the crate's `tests/it` is declared nowhere.
///
/// A file is declared by `mod <name>;` in `main.rs` or in another file there (a support module
/// that tests include by path). Pass `env!("CARGO_MANIFEST_DIR")`.
///
/// # Panics
///
/// Panics, naming the files, when a module is not declared.
pub fn assert_all_modules_included(manifest_dir: &str) {
    let dir = Path::new(manifest_dir).join("tests").join("it");
    let everywhere = sources(&dir)
        .iter()
        .map(|path| read(path))
        .collect::<Vec<_>>()
        .join("\n");
    let missing: Vec<String> = entries(&dir)
        .into_iter()
        .filter_map(|path| module_of(&path))
        .filter(|name| !everywhere.contains(&format!("mod {name};")))
        .collect();
    assert!(
        missing.is_empty(),
        "tests/it holds modules that nothing includes: {}",
        missing.join(", ")
    );
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

fn entries(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("listing {}: {e}", dir.display()))
        .flatten()
        .map(|entry| entry.path())
        .collect()
}

/// Every `.rs` file under `dir`.
fn sources(dir: &Path) -> Vec<PathBuf> {
    entries(dir)
        .into_iter()
        .flat_map(|path| {
            if path.is_dir() {
                sources(&path)
            } else {
                vec![path]
            }
        })
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect()
}

/// The module a top-level entry stands for: a `.rs` file other than `main.rs`. A directory next
/// to it holds the children of that module, or data.
fn module_of(path: &Path) -> Option<String> {
    if path.is_dir() || path.extension()? != "rs" {
        return None;
    }
    let stem = path.file_stem()?.to_str()?.to_owned();
    (stem != "main").then_some(stem)
}
