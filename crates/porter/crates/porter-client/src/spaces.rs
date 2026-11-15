//! Why a Spaces call fails.

use crate::error::LinkError;

/// Why a Spaces call failed. More reasons may be added: match with a wildcard. A Space that is
/// not there, and an argument accountd refuses, are both `Link(LinkError::Invalid)`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SpacesError {
    /// The app made too many Spaces within the last minute; try again later.
    #[error("too many new Spaces")]
    TooMany,
    /// The call did not complete.
    #[error(transparent)]
    Link(#[from] LinkError),
}
