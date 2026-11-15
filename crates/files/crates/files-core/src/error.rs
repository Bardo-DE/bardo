//! Why a path or an item is refused, in the words a person reads. (A name's and a type's errors are
//! quire's, re-exported by the crate root.)

use ds_helpers::mime::NameError;

/// Why a path is not an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ItemError {
    /// Not an absolute path.
    #[error("That is not a full path.")]
    Path(#[from] PathError),
    /// The last part is not a usable name.
    #[error("{0}")]
    Name(#[from] NameError),
    /// The path has no last part (`/`, or one that ends in `..`).
    #[error("That path names no item.")]
    NoName,
}

/// A path is not a folder location: it is relative, which means nothing without a current folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not an absolute path")]
pub struct PathError;
