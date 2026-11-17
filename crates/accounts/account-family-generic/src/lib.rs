//! The Generic family: a mail server or a DAV server with a password (or an app password) and
//! no provider of its own. `generic-imap` is found by autoconfig, SRV and MX from the address,
//! `generic-dav` by `.well-known` from the server's name, and a brand file with fixed endpoints
//! (`fastmail`, `icloud`, `yahoo`, `gmx`) asks for an address and a password and uses the servers
//! the file names.
//!
//! Like Nextcloud's, a session mints no token (the relay presents the password), `discover`
//! answers what the provider file declares, and `revoke` has nothing to do at a server that
//! has no such call: removing the account wipes the password locally.

mod dav;
mod dns;
mod jmap;
mod login;
mod lookup;
mod mail;
mod sign_in;

/// What a account-generic-proto error means for a sign-in. Plain `http` to another computer is a
/// server that cannot be reached safely.
#[expect(
    clippy::needless_pass_by_value,
    reason = "passed by name to `map_err`, which hands the error over by value"
)]
fn fault(error: account_generic_proto::Error) -> account_sheet::SignInFault {
    use account_sheet::SignInFault;
    match error {
        account_generic_proto::Error::Refused => SignInFault::Refused,
        account_generic_proto::Error::Unreachable | account_generic_proto::Error::Insecure => {
            SignInFault::Unreachable
        }
        account_generic_proto::Error::Unreadable
        | account_generic_proto::Error::Url(_)
        | account_generic_proto::Error::Uri(_)
        | account_generic_proto::Error::Request(_)
        | account_generic_proto::Error::Text(_)
        | account_generic_proto::Error::Endpoint(_) => SignInFault::Unreadable,
    }
}

use account_capability::Claim;
use account_core::AccountId;
use account_core::{Account, Audience, Credential, IssuedToken};
use account_discover::Dns;
use account_family_kit::Conversation;
use account_family_kit::Io;
use account_family_kit::password::secret_presented;
use account_generic_proto::declared;
use http_core::{NoSleep, SharedHttp};
use account_provider::{
    Presented, Provider, ProviderError, ProviderSession, ProviderSet, ProviderSpec, RevokeOutcome,
    SignInStart,
};
use std::marker::PhantomData;

pub use dns::SharedDns;
pub use lookup::{DavLookup, FixedLookup, MailLookup};
pub use sign_in::{GenericFlow, Located, Lookup, Mid, Outcome, Seams};

/// The Generic family's provider for the accounts `L` finds the servers of.
#[derive(Debug, Clone)]
pub struct GenericProvider<L = MailLookup> {
    spec: ProviderSpec,
    io: Io,
    dns: SharedDns,
    providers: ProviderSet,
    login: login::SharedLogin,
    lookup: PhantomData<fn() -> L>,
}

impl<L> GenericProvider<L> {
    /// The provider serving the accounts of `spec`, dialling through `http` and resolving
    /// through `dns`. It knows no other provider, so every address is searched for.
    pub fn new(spec: ProviderSpec, http: SharedHttp, dns: impl Dns + 'static) -> Self {
        Self {
            spec,
            io: Io::new(http, NoSleep),
            dns: SharedDns::new(dns),
            providers: ProviderSet::default(),
            login: login::SharedLogin::platform(),
            lookup: PhantomData,
        }
    }

    /// The same provider that tries a typed password at the mail server through `connect`
    /// (default: TCP and TLS over the platform's roots). A test's is a connector that trusts a
    /// scratch CA.
    #[must_use]
    pub fn with_connect(self, connect: impl account_proxy::Connect + 'static) -> Self {
        Self {
            login: login::SharedLogin::new(connect),
            ..self
        }
    }

    /// The same provider that lets these providers claim an address first (a Fastmail address
    /// is Fastmail's to sign in).
    #[must_use]
    pub fn with_providers(self, providers: ProviderSet) -> Self {
        Self { providers, ..self }
    }
}

/// An open generic account.
#[derive(Debug)]
pub struct GenericSession {
    account: AccountId,
}

impl GenericSession {
    /// The account it is open for.
    #[must_use]
    pub fn account(&self) -> &AccountId {
        &self.account
    }
}

impl<L: Lookup> Provider for GenericProvider<L> {
    type Session = GenericSession;
    type SignIn = Conversation<GenericFlow<L>>;

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
        Ok(declared(&self.spec))
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn open(
        &self,
        account: &AccountId,
        presented: Presented,
    ) -> Result<GenericSession, ProviderError> {
        secret_presented(&presented)?;
        Ok(GenericSession {
            account: account.clone(),
        })
    }

    fn sign_in(&self, start: SignInStart) -> Result<Self::SignIn, ProviderError> {
        Ok(Conversation::new(GenericFlow::new(
            self.io.clone(),
            self.dns.clone(),
            self.providers.clone(),
            self.login.clone(),
            self.spec.clone(),
            start.mode,
        )))
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn revoke(
        &self,
        _account: &Account,
        presented: &Presented,
    ) -> Result<RevokeOutcome, ProviderError> {
        secret_presented(presented)?;
        Ok(RevokeOutcome::Unsupported)
    }
}

impl ProviderSession for GenericSession {
    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn access_token(&self, _audience: &Audience) -> Result<IssuedToken, ProviderError> {
        // A password is never handed to an app; the relay presents it.
        Err(ProviderError::Forbidden)
    }

    fn renewed(&self) -> Option<Credential> {
        None
    }
}
