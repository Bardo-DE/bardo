//! What the viewer was given: a path parsed once at the boundary, and the stamp that says which
//! version of the file was read.

mod file_name;
mod file_path;
mod input;
mod read_at;
mod stamp;

pub use file_name::FileName;
pub use file_path::FilePath;
pub use input::Input;
pub use read_at::{ReadAt, ReadAtStream};
pub use stamp::{ByteLen, FileStamp, ModTime};

/// A file the viewer was given (CLI, D-Bus, drop, launcher), parsed once at the boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Source {
    path: FilePath,
    stamp: FileStamp,
}

impl Source {
    /// The file at `path` as it stood at `stamp`.
    #[must_use]
    pub fn new(path: FilePath, stamp: FileStamp) -> Self {
        Source { path, stamp }
    }

    /// Where the file is.
    pub fn path(&self) -> &FilePath {
        &self.path
    }

    /// Which version of it was read.
    pub fn stamp(&self) -> FileStamp {
        self.stamp
    }
}
