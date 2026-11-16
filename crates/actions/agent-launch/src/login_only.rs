//! `LoginOnly`: the account seam for a launcher with no porter behind it. Only the `login` route
//! can work: the agent signs itself in and nothing is lent. Every route that needs a key or an
//! endpoint is refused, so an entry that asks for one fails to start instead of running without
//! what it was promised. It is what the live-eval harness hosts an agent with on its private bus,
//! where there is no accountd.

use crate::accounts::{AccountFault, Accounts, Heard, Issued, LoginAsk, OpenedEndpoint, RouteWish};
use account_capability::LoginOutcome;
use account_capability::{AgentProgram, AgentProtocol, EnvName};
use account_core::{DataClass, GrantId, ProcessCredentialId, SessionToken};

/// The seam with nothing behind it.
#[derive(Debug, Clone, Copy, Default)]
pub struct LoginOnly;

impl Accounts for LoginOnly {
    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn register(&self, _programs: &[AgentProgram]) -> Result<(), AccountFault> {
        Ok(())
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn begin_session(&self, _session: &SessionToken) -> Result<(), AccountFault> {
        Ok(())
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn end_session(&self, _session: &SessionToken) -> Result<(), AccountFault> {
        Ok(())
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn open_endpoint(
        &self,
        _program: &AgentProgram,
        _route: &RouteWish,
        _class: DataClass,
        _protocol: AgentProtocol,
    ) -> Result<OpenedEndpoint, AccountFault> {
        Err(AccountFault::Unreachable)
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn close_endpoint(&self, _session: &str) -> Result<(), AccountFault> {
        Ok(())
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn request_key(
        &self,
        _program: &AgentProgram,
        _class: DataClass,
        _session: &SessionToken,
    ) -> Result<GrantId, AccountFault> {
        Err(AccountFault::Unreachable)
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn issue(
        &self,
        _permit: &GrantId,
        _program: &AgentProgram,
        _key_env: &EnvName,
    ) -> Result<Issued, AccountFault> {
        Err(AccountFault::Unreachable)
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn revoke(&self, _id: &ProcessCredentialId) -> Result<(), AccountFault> {
        Ok(())
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn hear(&self) -> Option<Heard> {
        None
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn report(&self, _ask: &LoginAsk, _outcome: LoginOutcome) -> Result<(), AccountFault> {
        Ok(())
    }
}
