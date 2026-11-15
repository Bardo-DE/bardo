//! Why a name or a type is refused, or the database is missing, in the words a person reads.

/// Why a name was refused, in the words a person reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NameError {
    /// Nothing was typed.
    #[error("A name can't be empty.")]
    Empty,
    /// `.` or `..`, which name this folder and the one above it.
    #[error("The names \u{201c}.\u{201d} and \u{201c}..\u{201d} are reserved.")]
    Reserved,
    /// A `/`, which separates folders in a path.
    #[error("A name can't contain \u{201c}/\u{201d}.")]
    Slash,
    /// A NUL character, which no Linux name may hold.
    #[error("A name can't contain a null character.")]
    Nul,
    /// Over 255 bytes.
    #[error("That name is too long.")]
    TooLong,
}

/// A text that is not a MIME type: not one `type/subtype` pair of non-empty parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not a type/subtype pair")]
pub struct MimeError;

/// The file type database could not be loaded: no data folder held any file name patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("No file type database was found.")]
pub struct LoadError;
