//! The real the accounts link: the accounts service's launcher interface
//! (`account_client_dbus::Launcher`) and its `org.bardo.Inference1.Agents` (`OpenEndpoint`,
//! `CloseEndpoint`) over the bus.
//!
//! Built only with the `dbus` feature. A secret passes through here once, from the bus reply into
//! the caller's hands, and is in no log: `SecretText`'s `Debug` shows nothing, and no error carries
//! a value.

use crate::accounts::{
    AccountFault, Accounts, AskKind, Heard, Issued, LoginAsk, OpenedEndpoint, RouteWish,
};
use crate::config::EndpointKind;
use account_capability::LoginOutcome;
use account_capability::{AgentProgram, AgentProtocol, EnvName};
use account_client::{Changes, LinkError};
use account_client_dbus::{
    AskKind as AccountAskKind, Bus, ChildKey, Delivery as KeyDelivery, Launcher, LauncherError,
    LauncherRequest, Revoked,
};
use account_core::{DataClass, GrantId, ProcessCredentialId, SecretText, SessionToken};
use account_dbus::BusConnection;
use account_wire::LauncherWireError;
use inference_dbus::inference::{
    AgentsProxy, MODELS_ANY, MODELS_LISTED, ROUTE_ACCOUNT, ROUTE_MODEL,
};
use std::collections::HashMap;
use tokio::sync::Mutex;

/// The launcher's link to accountd and inferd.
pub struct DbusAccounts {
    launcher: Launcher,
    agents: AgentsProxy<'static>,
    requests: Mutex<Changes<LauncherRequest>>,
    revocations: Mutex<Changes<Revoked>>,
}

impl std::fmt::Debug for DbusAccounts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbusAccounts").finish_non_exhaustive()
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "passed by name to `map_err`, which hands the error over by value"
)]
fn fault(error: LauncherError) -> AccountError {
    let cause = AccountError::source_of(error.clone());
    match error {
        LauncherError::Link(LinkError::Unreachable) => AccountError::Unreachable(cause),
        LauncherError::Refused(_)
        | LauncherError::Link(LinkError::Denied)
        | LauncherError::Fault(LauncherWireError::OnceGrant) => AccountError::Refused(cause),
        _ => AccountError::Other(cause),
    }
}

fn bus_fault(error: zbus::Error) -> AccountError {
    match &error {
        zbus::Error::MethodError(..) => AccountError::Refused(AccountError::source_of(error)),
        zbus::Error::InputOutput(_) => AccountError::Unreachable(AccountError::source_of(error)),
        _ => AccountError::Other(AccountError::source_of(error)),
    }
}

/// A name for the bus, which the value's own text gives; anything else is no name.
#[derive(Debug, thiserror::Error)]
enum BadReplyError {
    #[error("the value does not serialise")]
    Serialise(#[source] serde_json::Error),
    #[error("the value is not a plain name")]
    NotText,
    #[error("the key was not given as a value")]
    NotValue,
}

fn slug<T: serde::Serialize>(value: &T) -> Result<String, AccountError> {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(s)) => Ok(s),
        Ok(_) => Err(AccountError::Other(AccountError::source_of(
            BadReplyError::NotText,
        ))),
        Err(error) => Err(AccountError::Other(AccountError::source_of(
            BadReplyError::Serialise(error),
        ))),
    }
}

impl DbusAccounts {
    /// Connects over `connection`, which accountd must know as the agent launcher. The request
    /// and revocation streams are subscribed before anything is registered or issued, so none
    /// is missed.
    ///
    /// # Errors
    ///
    /// Returns an [`AccountError`] when the bus cannot subscribe to the launcher's request and
    /// revocation streams, or does not reach the agents interface.
    pub async fn connect(connection: &BusConnection) -> Result<Self, AccountError> {
        let launcher = Bus::over(connection.clone()).launcher();
        let requests = launcher.requests().await.map_err(fault)?;
        let revocations = launcher.revocations().await.map_err(fault)?;
        let agents = AgentsProxy::new(connection).await.map_err(bus_fault)?;
        Ok(Self {
            launcher,
            agents,
            requests: Mutex::new(requests),
            revocations: Mutex::new(revocations),
        })
    }
}

impl Accounts for DbusAccounts {
    async fn register(&self, programs: &[AgentProgram]) -> Result<(), AccountError> {
        self.launcher.register(programs).await.map_err(fault)
    }

    async fn begin_session(&self, session: &SessionToken) -> Result<(), AccountError> {
        self.launcher.begin_session(session).await.map_err(fault)
    }

    async fn end_session(&self, session: &SessionToken) -> Result<(), AccountError> {
        self.launcher.end_session(session).await.map_err(fault)
    }

    async fn open_endpoint(
        &self,
        program: &AgentProgram,
        route: &RouteWish,
        class: DataClass,
        protocol: AgentProtocol,
    ) -> Result<OpenedEndpoint, AccountError> {
        let kind = match route.kind {
            EndpointKind::Model => ROUTE_MODEL,
            EndpointKind::Account => ROUTE_ACCOUNT,
        };
        let models = if route.models.is_empty() {
            MODELS_ANY
        } else {
            MODELS_LISTED
        };
        let arg = (
            kind.to_owned(),
            route.id.clone(),
            models.to_owned(),
            route.models.clone(),
        );
        let class = slug(&class)?;
        let protocol = slug(&protocol)?;
        let (session, _scheme, host, port, base_url, token, _protocol) = self
            .agents
            .open_endpoint(
                program.as_str(),
                &arg,
                &class,
                &[protocol.as_str()],
                &HashMap::new(),
            )
            .await
            .map_err(bus_fault)?;
        Ok(OpenedEndpoint {
            session,
            host,
            port,
            base_url,
            token: SecretText::new(token),
        })
    }

    async fn close_endpoint(&self, session: &str) -> Result<(), AccountError> {
        self.agents.close_endpoint(session).await.map_err(bus_fault)
    }

    async fn request_key(
        &self,
        program: &AgentProgram,
        class: DataClass,
        session: &SessionToken,
    ) -> Result<GrantId, AccountError> {
        self.launcher
            .request_agent_key(program, class, Some(session))
            .await
            .map(|candidate| candidate.grant)
            .map_err(fault)
    }

    async fn issue(
        &self,
        permit: &GrantId,
        program: &AgentProgram,
        key_env: &EnvName,
    ) -> Result<Issued, AccountError> {
        let credential = self
            .launcher
            .issue_credential(permit, program)
            .await
            .map_err(fault)?;
        let ChildKey::Value { value: key, .. } = credential
            .child_key(key_env, KeyDelivery::Value)
            .map_err(|error| AccountError::Other(AccountError::source_of(error)))?
        else {
            return Err(AccountError::Other(AccountError::source_of(
                BadReplyError::NotValue,
            )));
        };
        Ok(Issued {
            id: credential.id().clone(),
            key,
        })
    }

    async fn revoke(&self, id: &ProcessCredentialId) -> Result<(), AccountError> {
        self.launcher.revoke_credential(id).await.map_err(fault)
    }

    async fn hear(&self) -> Option<Heard> {
        let mut requests = self.requests.lock().await;
        let mut revocations = self.revocations.lock().await;
        loop {
            // A message that is not well formed is not answered; the next is heard.
            tokio::select! {
                ask = requests.next() => {
                    if let Ok(ask) = ask? {
                        return Some(Heard::Ask(LoginAsk {
                            kind: match ask.kind {
                                AccountAskKind::Login => AskKind::Login,
                                AccountAskKind::Logout => AskKind::Logout,
                            },
                            request: ask.request,
                            account: ask.account,
                            program: ask.program,
                        }));
                    }
                }
                gone = revocations.next() => {
                    if let Ok(revoked) = gone? {
                        return Some(Heard::Revoked(revoked.id));
                    }
                }
            }
        }
    }

    async fn report(&self, ask: &LoginAsk, outcome: LoginOutcome) -> Result<(), AccountError> {
        let result = match ask.kind {
            AskKind::Login => self.launcher.report_login(&ask.request, outcome).await,
            AskKind::Logout => self.launcher.report_logout(&ask.request, outcome).await,
        };
        result.map_err(fault)
    }
}
