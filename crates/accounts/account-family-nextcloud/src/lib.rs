//! The Nextcloud family: Login Flow v2 (a browser sign-in that mints an app password), OCS
//! discovery, `WebDAV`, `CalDAV`, `CardDAV` and the Notes API, over one app password.
//!
//! A password never leaves accountd, so a session mints no token; apps reach the servers through
//! the authenticated relay. The trait's `discover` and `revoke` are handed the account, whose
//! endpoints name the server and the login; the app password is what it presents.

mod discover;
mod flow;
mod revoke;
mod sign_in;

use account_capability::Claim;
use account_core::AccountId;
use account_core::{Account, Audience, Credential, EndpointUrl, IssuedToken};
use account_family_kit::Conversation;
use account_family_kit::password::password_of;
use account_family_kit::{Io, Pacing};
use account_http::{SharedHttp, Sleep};
use account_nextcloud_proto::paths::server_of;
use account_provider::{
    Presented, Provider, ProviderError, ProviderSession, ProviderSpec, RevokeOutcome, SignInStart,
};
use account_sheet::SignInFault;
use discover::Who;

pub use sign_in::NextcloudFlow;

/// What a account-nextcloud-proto error means for a sign-in. Plain `http` to another computer is a
/// server that cannot be reached safely.
fn fault(error: account_nextcloud_proto::Error) -> SignInFault {
    match error {
        account_nextcloud_proto::Error::Refused => SignInFault::Refused,
        account_nextcloud_proto::Error::Unreachable | account_nextcloud_proto::Error::Insecure => {
            SignInFault::Unreachable
        }
        _ => SignInFault::Unreadable,
    }
}

/// The Nextcloud family's provider.
#[derive(Debug, Clone)]
pub struct NextcloudProvider {
    spec: ProviderSpec,
    io: Io,
}

impl NextcloudProvider {
    /// The provider serving the accounts of `spec`, dialling through `http` and waiting on
    /// `sleep` between the polls of a sign-in.
    pub fn new(spec: ProviderSpec, http: SharedHttp, sleep: impl Sleep + 'static) -> Self {
        Self {
            spec,
            io: Io::new(http, sleep),
        }
    }

    /// The same provider polling at this pace.
    #[must_use]
    pub fn with_pacing(self, pacing: Pacing) -> Self {
        Self {
            io: Io { pacing, ..self.io },
            ..self
        }
    }

    /// Where the provider file says the Nextcloud is, if it names one: the root of its DAV
    /// endpoint rows (a deployment's own file; a test's fake).
    fn fixed_server(&self) -> Option<EndpointUrl> {
        self.spec.capabilities.iter().find_map(|row| {
            let at = row.endpoint.as_ref()?;
            let root = at.split("/remote.php/").next()?;
            (root.len() < at.len())
                .then(|| EndpointUrl::parse(root.trim_end_matches('/')).ok())
                .flatten()
        })
    }
}

/// An open Nextcloud account.
#[derive(Debug)]
pub struct NextcloudSession {
    account: AccountId,
}

impl NextcloudSession {
    /// The account it is open for.
    #[must_use]
    pub fn account(&self) -> &AccountId {
        &self.account
    }
}

impl Provider for NextcloudProvider {
    type Session = NextcloudSession;
    type SignIn = Conversation<NextcloudFlow>;

    fn spec(&self) -> &ProviderSpec {
        &self.spec
    }

    async fn discover(
        &self,
        account: &Account,
        presented: &Presented,
    ) -> Result<Vec<Claim>, ProviderError> {
        let (server, login) = server_of(&account.endpoints).ok_or(ProviderError::Unreadable)?;
        let who = Who {
            server,
            login,
            password: password_of(presented)?.clone(),
        };
        discover::discover(&self.io, &who)
            .await
            .map(|found| found.claims)
            .map_err(|fault| match fault {
                SignInFault::Refused => ProviderError::Unauthorized,
                SignInFault::Unreachable => ProviderError::Unreachable,
                _ => ProviderError::Unreadable,
            })
    }

    async fn open(
        &self,
        account: &AccountId,
        presented: Presented,
    ) -> Result<NextcloudSession, ProviderError> {
        password_of(&presented)?;
        Ok(NextcloudSession {
            account: account.clone(),
        })
    }

    fn sign_in(&self, start: SignInStart) -> Result<Self::SignIn, ProviderError> {
        Ok(Conversation::new(NextcloudFlow::new(
            self.io.clone(),
            start.mode,
            self.fixed_server(),
        )))
    }

    async fn revoke(
        &self,
        account: &Account,
        presented: &Presented,
    ) -> Result<RevokeOutcome, ProviderError> {
        let (server, login) = server_of(&account.endpoints).ok_or(ProviderError::Unreadable)?;
        revoke::revoke(&self.io, &server, &login, password_of(presented)?).await
    }
}

impl ProviderSession for NextcloudSession {
    async fn access_token(&self, _audience: &Audience) -> Result<IssuedToken, ProviderError> {
        // An app password is never handed to an app; the relay presents it.
        Err(ProviderError::Forbidden)
    }

    fn renewed(&self) -> Option<Credential> {
        None
    }
}
