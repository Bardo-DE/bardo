//! A private folder per test, removed on drop.

use std::path::{Path, PathBuf};

use tempfile::TempDir;

pub struct Scratch(TempDir);

impl Scratch {
    /// A fresh empty folder under the system temporary folder; `test` only names it for a human.
    pub fn new(test: &str) -> Scratch {
        let dir = tempfile::Builder::new()
            .prefix(&format!("chordkit-file-{test}-"))
            .tempdir()
            .unwrap_or_else(|e| panic!("{test}: {e}"));
        Scratch(dir)
    }

    pub fn path(&self) -> &Path {
        self.0.path()
    }

    /// Writes `bytes` to `name` in the folder and returns its path.
    pub fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.path().join(name);
        std::fs::write(&path, bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        path
    }
}
