//! account-provider's errors: a refused provider file, and a provider call that failed.

use account_core::ProviderId;

/// Why a provider file was refused. accountd logs it and skips the file; the rest load.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProviderFileError {
    /// Not valid TOML, or not the provider schema.
    #[error("provider file: {0}")]
    Syntax(String),
    /// An OAuth kind without an issuer, or an issuer on a kind that has none.
    #[error("provider {0}: auth issuer does not match its kind")]
    IssuerMismatch(ProviderId),
    /// No capability rows.
    #[error("provider {0}: declares no capability")]
    NoCapabilities(ProviderId),
    /// AI capability rows without an `[ai]` table, or an `[ai]` table without them.
    #[error("provider {0}: [ai] does not match its capabilities")]
    AiSpecMismatch(ProviderId),
    /// A language-model row whose `wire` is a different request format from its `family`'s.
    #[error("provider {0}: a language-model row's wire is not its family's")]
    WireFamily(ProviderId),
    /// Agent rows that break the rules: they are served by the `acp_agent` family and name each
    /// program once, and an `agent_login` provider declares nothing else.
    #[error("provider {0}: agent rows are not well formed")]
    AgentRows(ProviderId),
}

/// The error under a failed call, shared so [`ProviderError`] stays `Clone`.
pub type Cause = std::sync::Arc<dyn std::error::Error + Send + Sync>;

/// Why a call to a provider failed, in terms the account's state can take.
///
/// Two errors are equal when they are the same kind of failure: [`Self::Down`] is the kind
/// [`Self::Unreachable`] is, and [`Self::Garbled`] the kind [`Self::Unreadable`] is; the cause
/// does not count.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ProviderError {
    /// The credential was refused: the account needs signing in again.
    #[error("credential refused")]
    Unauthorized,
    /// The account's organisation or the provider forbids this (a tenant consent 403).
    #[error("forbidden")]
    Forbidden,
    /// The server could not be reached.
    #[error("unreachable")]
    Unreachable,
    /// The server answered something this family cannot read.
    #[error("unreadable answer")]
    Unreadable,
    /// The account was not given this access (it is asked when first used, or the person left
    /// it unticked): the credential is good, and only a sign-in that asks for the access
    /// (`SignInMode::Extend`) changes the answer.
    #[error("access not given")]
    NeedsConsent,
    /// The server could not be reached, and why.
    #[error("unreachable")]
    Down(#[source] Cause),
    /// The server answered something this family cannot read, and why.
    #[error("unreadable answer")]
    Garbled(#[source] Cause),
}

impl ProviderError {
    /// The server could not be reached because of `cause`.
    pub fn down(cause: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Down(Cause::from(cause.into()))
    }

    /// The answer could not be read because of `cause`.
    pub fn garbled(cause: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Garbled(Cause::from(cause.into()))
    }

    fn kind(&self) -> u8 {
        match self {
            Self::Unauthorized => 0,
            Self::Forbidden => 1,
            Self::Unreachable | Self::Down(_) => 2,
            Self::Unreadable | Self::Garbled(_) => 3,
            Self::NeedsConsent => 4,
        }
    }
}

impl PartialEq for ProviderError {
    fn eq(&self, other: &Self) -> bool {
        self.kind() == other.kind()
    }
}

impl Eq for ProviderError {}

/// What a failed call says to the person signing in: the one mapping every family uses.
impl From<ProviderError> for account_sheet::SignInFault {
    fn from(error: ProviderError) -> Self {
        use account_sheet::SignInFault;
        match error {
            ProviderError::Unauthorized => SignInFault::Refused,
            ProviderError::Forbidden => SignInFault::Forbidden,
            ProviderError::Unreachable | ProviderError::Down(_) => SignInFault::Unreachable,
            // access that was not given ends as unreadable, as does any variant a newer the accounts service adds
            ProviderError::Unreadable | ProviderError::Garbled(_) | ProviderError::NeedsConsent => {
                SignInFault::Unreadable
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProviderError;
    use account_sheet::SignInFault;

    #[test]
    fn a_failed_call_is_a_fault_the_person_can_read() {
        let cases = [
            (ProviderError::Unauthorized, SignInFault::Refused),
            (ProviderError::Forbidden, SignInFault::Forbidden),
            (ProviderError::Unreachable, SignInFault::Unreachable),
            (ProviderError::Unreadable, SignInFault::Unreadable),
            (ProviderError::NeedsConsent, SignInFault::Unreadable),
        ];
        for (error, want) in cases {
            let label = format!("{error:?}");
            assert_eq!(SignInFault::from(error), want, "{label}");
        }
    }
}
