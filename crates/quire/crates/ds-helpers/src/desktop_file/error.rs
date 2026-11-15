//! Why a desktop file was not read.

use crate::ReadCap;

/// A desktop file that was refused whole.
///
/// The text is longer than the caller's cap; nothing of it was read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the file is {size} bytes, over the limit of {cap}")]
pub struct DesktopFileError {
    /// The text's length in bytes.
    pub size: u64,
    /// The cap it went over.
    pub cap: ReadCap,
}
