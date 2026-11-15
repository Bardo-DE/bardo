//! Why a call to accountd's Keys fails.

use crate::error::LinkError;
use porter_wire::Refusal;

/// Why a call to accountd's Keys failed. More reasons may be added: match with a wildcard.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AiKeysError {
    /// accountd refused for a reason the person or the machine gave: the person closed the sheet
    /// (`Dismissed`) or refused (`Denied`), no key fits (`NoFittingAccount`), a key must be signed
    /// in again (`NeedsReauth`), or the service cannot do it now (`Unavailable`).
    #[error("refused")]
    Refused(Refusal),
    /// The call did not complete.
    #[error(transparent)]
    Link(#[from] LinkError),
}
