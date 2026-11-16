//! Scratch directories: tests never touch the real XDG directories.

use memory_core::Dirs;
use tempfile::TempDir;

/// Four scratch roots under one temporary directory, removed when dropped.
#[derive(Debug)]
pub struct Scratch {
    dir: TempDir,
    dirs: Dirs,
}

impl Scratch {
    /// New empty roots.
    ///
    /// # Errors
    ///
    /// Returns the operating system's error when the temporary directory cannot be created.
    pub fn new() -> std::io::Result<Self> {
        let dir = TempDir::new()?;
        let root = dir.path();
        let dirs = Dirs::new(
            root.join("data"),
            root.join("cache"),
            root.join("config"),
            root.join("run"),
        );
        Ok(Self { dir, dirs })
    }

    /// The injected directories.
    #[must_use]
    pub fn dirs(&self) -> &Dirs {
        &self.dirs
    }

    /// The temporary directory holding all four.
    #[must_use]
    pub fn root(&self) -> &std::path::Path {
        self.dir.path()
    }
}
