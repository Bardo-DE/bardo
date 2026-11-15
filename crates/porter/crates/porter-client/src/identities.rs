//! What an Identities call answers and why it fails.

use crate::error::LinkError;
use porter_core::ProviderId;
use porter_core::{IdentityAddress, IdentityId};
use porter_wire::Refusal;

/// Why an Identities call failed. More reasons may be added: match with a wildcard.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdentitiesError {
    /// The person closed the sheet without picking an identity.
    #[error("cancelled")]
    Cancelled,
    /// No identity fits: none is signed in on this machine, or none of the provider asked for.
    #[error("no identity")]
    NoIdentity,
    /// This app has an identity picker open already.
    #[error("an identity picker is open")]
    Busy,
    /// accountd refused the call for a reason it names.
    #[error("refused")]
    Refused(Refusal),
    /// The password of the identity is gone: it needs signing in again.
    #[error("needs sign-in")]
    NeedsSignIn,
    /// accountd is there but cannot do it now (no secret store, no way to show the picker).
    #[error("unavailable")]
    Unavailable,
    /// The call did not complete.
    #[error(transparent)]
    Link(#[from] LinkError),
}

/// The identity the person picked in `offer`: who to continue as. It holds no token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picked {
    /// The identity's id, which `register_use` and `take_password` take.
    pub id: IdentityId,
    /// The address to continue as.
    pub address: IdentityAddress,
    /// The provider's id.
    pub provider: ProviderId,
}

impl Picked {
    /// The identity `id`, at `address`, of `provider`.
    pub fn new(id: IdentityId, address: IdentityAddress, provider: ProviderId) -> Self {
        Self {
            id,
            address,
            provider,
        }
    }
}
