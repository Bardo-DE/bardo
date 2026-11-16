//! The errors of this crate.

/// Why files were not opened, in the words the window shows (a toast).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct OpenFailure(pub String);

/// The disks could not be listed or watched: this computer has no service that lists disks, or it
/// did not answer. The window shows none; it tells nobody.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("The disks can't be listed right now.")]
pub struct VolumeError;

/// The window closed before it answered a question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Files was closed before it could answer.")]
pub struct ReadError;
