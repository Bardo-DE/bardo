//! Why a call to the Broker fails.

use crate::error::LinkError;
use account_wire::Refusal;

/// Why a call to the Broker failed. More reasons may be added: match with a wildcard.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AiAccountsError {
    /// accountd refused for a reason an app also meets: `UnknownGrant` (no such permit, or a
    /// once permit already spent), `AudienceNotGranted`, `Denied` (the person turned the account
    /// off), `NeedsReauth` (the key is gone or must be signed in again) or `Unavailable`.
    #[error("refused")]
    Refused(Refusal),
    /// The call did not complete: accountd is not there, did not know the caller as the AI broker,
    /// refused an argument, or sent something that is not the accounts service's protocol.
    #[error(transparent)]
    Link(#[from] LinkError),
}
