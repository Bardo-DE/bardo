//! The root directory the app gives, and the layout under it.

use memory_core::Dirs;
use std::path::{Path, PathBuf};

/// The one directory an app hands to memory.
///
/// Everything lives below it: `data/` (logs, files,
/// `spaces.toml`), `cache/` (rebuildable indexes), `config/` and `run/` (unused by the portable
/// backend, kept so [`Dirs`] stays whole).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Root(PathBuf);

impl Root {
    /// The app's directory for memory.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// The directory itself.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }

    /// The layout, as the service's [`Dirs`].
    #[must_use]
    pub fn dirs(&self) -> Dirs {
        Dirs::new(
            self.0.join("data"),
            self.0.join("cache"),
            self.0.join("config"),
            self.0.join("run"),
        )
    }
}
