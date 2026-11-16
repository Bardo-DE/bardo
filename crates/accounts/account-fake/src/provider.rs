//! A provider that answers from its declaration and mints predictable tokens.

use account_capability::{Claim, Offer, Provenance, Subject};
use account_core::AccountId;
use account_core::{
    Account, AccountLabel, Audience, Credential, IssuedToken, Restriction, SecretPurpose,
    SecretText, TokenKind,
};
use account_provider::{
    Presented, Provider, ProviderError, ProviderSession, ProviderSpec, RevokeOutcome, SignIn,
    SignInMode, SignInStart, SignInStep, Signed, parse_provider,
};
use account_sheet::{SignInFault, SignInInput};
use base_time::UnixSeconds;

/// A provider over one parsed provider file.
#[derive(Debug, Clone)]
pub struct FakeProvider {
    spec: ProviderSpec,
    refuses_refresh: bool,
}

impl FakeProvider {
    /// The provider the file text declares. Panics on a bad file: the files are this crate's.
    ///
    /// # Panics
    ///
    /// If `text` is not a valid provider file: the fake is built from fixed text in a test, so a
    /// bad file is a mistake in the test.
    #[must_use]
    pub fn from_file(text: &str) -> Self {
        let spec = parse_provider(text).unwrap_or_else(|e| panic!("fake provider file: {e}"));
        Self {
            spec,
            refuses_refresh: false,
        }
    }

    /// The same provider with its issuer refusing every stored credential (`invalid_grant`):
    /// `open` answers `Unauthorized`, as a revoked refresh token does.
    #[must_use]
    pub fn refusing_refresh(mut self) -> Self {
        self.refuses_refresh = true;
        self
    }
}

/// The self-hosted cloud (Storage + Calendar, app password).
#[must_use]
pub fn cloud_provider() -> FakeProvider {
    FakeProvider::from_file(include_str!("../providers/fake-cloud.toml"))
}

/// The mail host (Mail, OAuth).
#[must_use]
pub fn mail_provider() -> FakeProvider {
    FakeProvider::from_file(include_str!("../providers/fake-mail.toml"))
}

/// The local runtime (Llm, on device).
#[must_use]
pub fn llm_provider() -> FakeProvider {
    FakeProvider::from_file(include_str!("../providers/fake-llm.toml"))
}

/// An open fake account.
#[derive(Debug, Clone)]
pub struct FakeSession {
    account: AccountId,
}

/// A sign-in that needs nothing from the person.
///
/// `Start` signs in at once with a fixed password, and anything after it is a refusal.
///
/// A real family asks, waits and reviews. Signing
/// an account in again, or asking it for one more access, answers its servers as stored, as the
/// same person would; one more access is first shown for review, as every family shows it, and
/// signed in on `Confirm`.
#[derive(Debug)]
pub struct FakeSignIn {
    label: AccountLabel,
    claims: Vec<Claim>,
    endpoints: Vec<account_core::ServiceEndpoint>,
    reviews: bool,
}

impl FakeSignIn {
    fn signed(&self) -> SignInStep {
        SignInStep::Done(Signed::new(
            self.label.clone(),
            vec![(
                SecretPurpose::Password,
                Credential::Password(SecretText::new("fake-signed-in")),
            )],
            self.claims.clone(),
            self.endpoints.clone(),
            Restriction::none(),
        ))
    }
}

impl SignIn for FakeSignIn {
    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn next(&mut self, input: SignInInput) -> SignInStep {
        match input {
            SignInInput::Start if self.reviews => SignInStep::Review {
                claims: self.claims.clone(),
                endpoints: self.endpoints.clone(),
                restriction: Restriction::none(),
                label: self.label.clone(),
            },
            SignInInput::Start => self.signed(),
            SignInInput::Confirm(_) if self.reviews => self.signed(),
            _ => SignInStep::Failed(SignInFault::Cancelled),
        }
    }
}

impl Provider for FakeProvider {
    type Session = FakeSession;
    type SignIn = FakeSignIn;

    fn spec(&self) -> &ProviderSpec {
        &self.spec
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn discover(
        &self,
        _account: &Account,
        _presented: &Presented,
    ) -> Result<Vec<Claim>, ProviderError> {
        Ok(self
            .spec
            .capabilities
            .iter()
            .map(|row| Claim {
                subject: Subject::Account,
                offer: Offer::Present(row.capability.clone()),
                provenance: Provenance::Discovered,
            })
            .collect())
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn open(
        &self,
        account: &AccountId,
        presented: Presented,
    ) -> Result<FakeSession, ProviderError> {
        let needs_secret = self.auth_kind().facts().secret.is_some();
        match (needs_secret, presented) {
            (true, Presented::Anonymous) => Err(ProviderError::Unauthorized),
            (_, Presented::Credential(_)) if self.refuses_refresh => {
                Err(ProviderError::Unauthorized)
            }
            _ => Ok(FakeSession {
                account: account.clone(),
            }),
        }
    }

    fn sign_in(&self, start: SignInStart) -> Result<FakeSignIn, ProviderError> {
        let reviews = matches!(start.mode, SignInMode::Extend { .. });
        let endpoints = match start.mode {
            SignInMode::Reauthenticate { endpoints, .. } | SignInMode::Extend { endpoints, .. } => {
                endpoints
            }
            _ => Vec::new(),
        };
        Ok(FakeSignIn {
            endpoints,
            reviews,
            label: AccountLabel(self.spec.label.clone()),
            claims: self
                .spec
                .capabilities
                .iter()
                .map(|row| Claim {
                    subject: Subject::Account,
                    offer: Offer::Present(row.capability.clone()),
                    provenance: Provenance::Declared,
                })
                .collect(),
        })
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn revoke(
        &self,
        _account: &Account,
        _presented: &Presented,
    ) -> Result<RevokeOutcome, ProviderError> {
        Ok(RevokeOutcome::Revoked)
    }
}

impl ProviderSession for FakeSession {
    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn access_token(&self, audience: &Audience) -> Result<IssuedToken, ProviderError> {
        Ok(IssuedToken::new(
            TokenKind::Bearer,
            SecretText::new(format!("fake:{}:{}", self.account, audience.0)),
            UnixSeconds(crate::world::NOW.0 + 3600),
        ))
    }

    fn renewed(&self) -> Option<account_core::Credential> {
        None
    }
}
