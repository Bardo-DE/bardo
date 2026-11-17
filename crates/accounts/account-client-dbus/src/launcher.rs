//! The launcher's side of an agent's login (accountd's `Launcher` over the bus; it is this crate's and
//! not `AccountClient`'s, because the key it hands over is a sealed memfd, a Linux descriptor) (design/31 R7-R9, C5, C6). The agent launcher
//! (bardo-agent-acp) registers the programs it runs, hears when accountd asks it to sign an agent in or
//! out, has the agent do it, and reports only a coarse [`LoginOutcome`]. Nothing of the login
//! itself (a token, a URL, a code) crosses to the accounts service: there is no way to send one.
//!
//! The launcher talks to accountd (`org.bardo.Accounts1`), which holds the AI keys: whoever uses a
//! credential holds it. An agent's yes to use one of the person's keys is a permit
//! ([`GrantId`]), asked for with [`Launcher::request_agent_key`].
//!
//! For an agent that cannot be pointed at inferd the launcher can also be handed an API key for
//! the process it spawns ([`Launcher::issue_credential`]; the duties and the residual risk are in
//! [`crate::credential`]).
//!
//! The caller must be known to accountd as the agent launcher (`CallerRole::AgentLauncher`, a row
//! of accountd's caller table a machine writes); any other caller is `Link(LinkError::Denied)`.
//!
//! ```ignore
//! let launcher = Bus::over(connection).launcher();
//! let mut requests = launcher.requests().await?;       // subscribe first
//! launcher.register(&[AgentProgram::parse("claude-code")?]).await?;
//! while let Some(ask) = requests.next().await {
//!     let ask = ask?;
//!     let outcome = run_the_agents_own_login(&ask).await;   // the agent's, never the accounts service's
//!     match ask.kind {
//!         AskKind::Login => launcher.report_login(&ask.request, outcome).await?,
//!         AskKind::Logout => launcher.report_logout(&ask.request, outcome).await?,
//!     }
//! }
//! ```

use crate::Bus;
use crate::credential::{ProcessCredential, Revoked};
use crate::failure::{launcher_error, link_error};
use account_capability::AgentProgram;
use account_capability::{LoginOutcome, LoginRequestId};
use account_client::{Changes, LinkError, Malformed, MalformedError};
use account_core::audit::CredentialEnd;
use account_core::{AccountId, GrantId};
use account_core::{Candidate, DataClass};
use account_core::{ProcessCredentialId, SessionToken};
use account_dbus::accounts::sheet::{SheetKind, reply_of};
use account_dbus::accounts::{
    Abandon, AgentLoginRequested, AgentLogoutRequested, LauncherProxy, ProcessCredentialRevoked,
    ServiceRoot, Sheet, SheetError,
};
use account_wire::AccountsReply;
use account_wire::AskKind;
use account_wire::LauncherWireError;
use account_wire::Refusal;
use base_ids::word::{from_word, word as word_of};
use futures_util::StreamExt;
use futures_util::stream::select;

/// Why a launcher call failed. More reasons may be added: match with a wildcard.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LauncherError {
    /// accountd refused the call for a reason of the launcher interface (a request, a program, a
    /// permit, a credential or a session that is not the connection's to use).
    #[error("{0}")]
    Fault(LauncherWireError),
    /// accountd refused the permit or the account for a reason an app also meets: `UnknownGrant`,
    /// `AudienceNotGranted` (the permit is not for `org.bardo.Agent.<program>`), `NeedsReauth`,
    /// `Denied` (the person turned the account off), `Unavailable`.
    #[error("refused")]
    Refused(Refusal),
    /// The call did not complete: accountd is not there, did not know the caller as the agent
    /// launcher, refused an argument, or sent something that is not the accounts service's protocol.
    #[error(transparent)]
    Link(#[from] LinkError),
}

/// One request accountd sent to this launcher alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherRequest {
    /// In or out.
    pub kind: AskKind,
    /// The id to report the outcome under.
    pub request: LoginRequestId,
    /// The agent account the request is for.
    pub account: AccountId,
    /// The program (one the launcher registered) that is to sign in or out.
    pub program: AgentProgram,
}

fn malformed(what: impl Into<MalformedError>) -> LauncherError {
    LauncherError::Link(LinkError::Malformed(what.into()))
}

impl Bus {
    async fn launcher_proxy(&self) -> Result<LauncherProxy<'static>, LauncherError> {
        self.accounts
            .proxy(&self.connection)
            .await
            .map_err(|e| LauncherError::Link(link_error(&e)))
    }
}

fn sheet_error(error: SheetError) -> LauncherError {
    match error {
        SheetError::Bus(error) => launcher_error(&error),
        SheetError::Gone => LauncherError::Link(LinkError::Unreachable),
        SheetError::Unreadable => malformed(Malformed::Sheet),
    }
}

fn request(
    kind: AskKind,
    request: &str,
    account: &str,
    program: &str,
) -> Result<LauncherRequest, MalformedError> {
    Ok(LauncherRequest {
        kind,
        request: LoginRequestId::parse(request).map_err(|e| Malformed::Id.because(e))?,
        account: AccountId::parse(account).map_err(|e| Malformed::Id.because(e))?,
        program: AgentProgram::parse(program).map_err(|e| Malformed::Id.because(e))?,
    })
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "a stream `map` callback, which is handed the signal by value"
)]
fn login(signal: AgentLoginRequested) -> Result<LauncherRequest, MalformedError> {
    let args = signal.args().map_err(|e| Malformed::Signal.because(e))?;
    request(AskKind::Login, &args.request, &args.account, &args.program)
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "a stream `map` callback, which is handed the signal by value"
)]
fn logout(signal: AgentLogoutRequested) -> Result<LauncherRequest, MalformedError> {
    let args = signal.args().map_err(|e| Malformed::Signal.because(e))?;
    request(AskKind::Logout, &args.request, &args.account, &args.program)
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "a stream `map` callback, which is handed the signal by value"
)]
fn revoked(signal: ProcessCredentialRevoked) -> Result<Revoked, MalformedError> {
    let args = signal.args().map_err(|e| Malformed::Signal.because(e))?;
    Ok(Revoked {
        id: ProcessCredentialId::parse(&args.id).map_err(|e| Malformed::Id.because(e))?,
        reason: from_word::<CredentialEnd>(&args.reason).ok_or(Malformed::Signal)?,
    })
}

/// The launcher's calls to accountd, over `bus`.
#[derive(Debug, Clone)]
pub struct Launcher {
    bus: Bus,
}

impl Bus {
    /// The agent launcher's calls to accountd over this connection.
    #[must_use]
    pub fn launcher(&self) -> Launcher {
        Launcher { bus: self.clone() }
    }
}

impl Launcher {
    /// Opens `session`, the span a permit "for this session only" lasts (the launcher names it,
    /// usually from an ACP session id). It belongs to this connection, which must have
    /// registered a program, and ends at [`Launcher::end_session`] or when the connection
    /// leaves the bus. A session another connection holds is [`LauncherWireError::SessionTaken`];
    /// beginning one this connection holds is a no-op.
    ///
    /// # Errors
    ///
    /// [`LauncherError`] when the session is taken, this connection registered no program, or the
    /// call fails.
    pub async fn begin_session(&self, session: &SessionToken) -> Result<(), LauncherError> {
        self.bus
            .launcher_proxy()
            .await?
            .begin_session(session.as_str())
            .await
            .map_err(|e| launcher_error(&e))
    }

    /// Ends a session this connection began: accountd removes the permits scoped to it and ends
    /// the process credentials under them (`CredentialEnd::SessionClosed`), telling this
    /// connection of each in [`Launcher::revocations`] so the launcher can end the processes. A
    /// session that is not open for this connection is [`LauncherWireError::UnknownSession`].
    ///
    /// # Errors
    ///
    /// [`LauncherError`] when the session is not open for this connection, or the call fails.
    pub async fn end_session(&self, session: &SessionToken) -> Result<(), LauncherError> {
        self.bus
            .launcher_proxy()
            .await?
            .end_session(session.as_str())
            .await
            .map_err(|e| launcher_error(&e))
    }

    /// Asks the person, on accountd's consent sheet titled for `program`, which of their AI keys
    /// the agent may use (interactive use). With `session`, an open session of this connection,
    /// the sheet also offers "This session only", and a permit made so is scoped to it. Returns
    /// the chosen account and its permit; a person who closes the sheet or refuses is
    /// [`LauncherError::Refused`] (`Dismissed`, `Denied`). The program must be one this
    /// connection registered.
    ///
    /// # Errors
    ///
    /// [`LauncherError`] when the person refuses or closes the sheet, the program was not
    /// registered by this connection, or the call fails.
    pub async fn request_agent_key(
        &self,
        program: &AgentProgram,
        class: DataClass,
        session: Option<&SessionToken>,
    ) -> Result<Candidate, LauncherError> {
        let proxy = self.bus.launcher_proxy().await?;
        let mut listener = Sheet::subscribe_on(&self.bus.connection, ServiceRoot::Accounts)
            .await
            .map_err(|e| launcher_error(&e))?;
        // Armed before the call: a dropped call may already have made the sheet.
        let mut abandon = Abandon(listener.closer(None));
        let path = proxy
            .request_agent_key(
                program.as_str(),
                word_of(&class),
                session.map_or("", SessionToken::as_str),
                "",
                &listener.options(),
            )
            .await;
        let path = match path {
            Ok(path) => path,
            Err(error) => {
                abandon.answered();
                return Err(launcher_error(&error));
            }
        };
        abandon.0 = listener.closer(Some(path.clone()));
        let (code, results) = listener.response(&path).await.map_err(sheet_error)?;
        abandon.answered();
        match reply_of(SheetKind::Choose, code, results)
            .map_err(|e| malformed(Malformed::Reply.because(e)))?
        {
            AccountsReply::Chosen(candidate) => Ok(candidate),
            AccountsReply::Refused(refusal) => Err(LauncherError::Refused(refusal)),
            _ => Err(malformed(Malformed::Reply)),
        }
    }

    /// An API key for a process this launcher is about to spawn for `program`, under `permit`
    /// (the person's yes for the app `org.bardo.Agent.<program>` to use a key, scope always, or
    /// "this session only" for a session this connection holds open), handed over on
    /// a sealed memfd to pass to the child, the one way a key is handed over. The program
    /// must be one this connection registered. A credential under a session permit ends when
    /// the session does ([`Launcher::end_session`], or this connection leaving).
    ///
    /// For an agent that cannot be pointed at inferd (P4, the metered route, needs none of this).
    /// The key is outside the accounts service's meter and the child can read its own key: see the module
    /// documentation of [`crate::credential`] for the launcher's duties.
    ///
    /// # Errors
    ///
    /// [`LauncherError`] when no key can be issued for the account, the program was not registered
    /// by this connection, or the call fails.
    pub async fn issue_credential(
        &self,
        permit: &GrantId,
        program: &AgentProgram,
    ) -> Result<ProcessCredential, LauncherError> {
        let (id, handle) = self
            .bus
            .launcher_proxy()
            .await?
            .issue_process_credential(permit.as_str(), program.as_str())
            .await
            .map_err(|e| launcher_error(&e))?;
        let id =
            ProcessCredentialId::parse(&id).map_err(|e| malformed(Malformed::Id.because(e)))?;
        Ok(ProcessCredential::new(id, handle.into()))
    }

    /// Ends a credential this connection was issued, because the process that held it exited or
    /// was killed: accountd forgets it. Not for a credential accountd
    /// ended itself (that is [`LauncherWireError::UnknownCredential`]).
    ///
    /// # Errors
    ///
    /// [`LauncherError`] when the credential is unknown to accountd, or the call fails.
    pub async fn revoke_credential(&self, id: &ProcessCredentialId) -> Result<(), LauncherError> {
        self.bus
            .launcher_proxy()
            .await?
            .revoke_process_credential(id.as_str())
            .await
            .map_err(|e| launcher_error(&e))
    }

    /// What accountd tells this connection when it ends a credential itself (the permit was
    /// revoked, the account removed): the launcher must end that process. Subscribe before
    /// issuing, so none is missed.
    ///
    /// # Errors
    ///
    /// [`LauncherError`] when subscribing to the signals fails.
    pub async fn revocations(&self) -> Result<Changes<Revoked>, LauncherError> {
        let stream = self
            .bus
            .launcher_proxy()
            .await?
            .receive_process_credential_revoked()
            .await
            .map_err(|e| launcher_error(&e))?;
        Ok(Changes::new(stream.map(revoked)))
    }

    /// Registers the programs this connection launches, for as long as it lives. A program
    /// another live connection holds is [`LauncherWireError::AlreadyRegistered`] and none of the list
    /// is taken; registering a program this connection holds is a no-op, and a later call adds
    /// more.
    ///
    /// # Errors
    ///
    /// [`LauncherError`] when another live connection holds one of the programs, or the call fails.
    pub async fn register(&self, programs: &[AgentProgram]) -> Result<(), LauncherError> {
        let names: Vec<&str> = programs.iter().map(AgentProgram::as_str).collect();
        self.bus
            .launcher_proxy()
            .await?
            .register_launcher(&names)
            .await
            .map_err(|e| launcher_error(&e))
    }

    /// The requests accountd sends this connection. Subscribe before registering, so none is
    /// missed.
    ///
    /// # Errors
    ///
    /// [`LauncherError`] when subscribing to the signals fails.
    pub async fn requests(&self) -> Result<Changes<LauncherRequest>, LauncherError> {
        let proxy = self.bus.launcher_proxy().await?;
        let logins = proxy
            .receive_agent_login_requested()
            .await
            .map_err(|e| launcher_error(&e))?;
        let logouts = proxy
            .receive_agent_logout_requested()
            .await
            .map_err(|e| launcher_error(&e))?;
        Ok(Changes::new(select(logins.map(login), logouts.map(logout))))
    }

    /// Reports what became of a login request. Only the connection that was sent it may.
    ///
    /// # Errors
    ///
    /// [`LauncherError`] when this connection was not sent the request, or the call fails.
    pub async fn report_login(
        &self,
        request: &LoginRequestId,
        outcome: LoginOutcome,
    ) -> Result<(), LauncherError> {
        let (word, reason) = outcome.to_wire();
        self.bus
            .launcher_proxy()
            .await?
            .report_agent_login(request.as_str(), word, reason)
            .await
            .map_err(|e| launcher_error(&e))
    }

    /// Reports what became of a logout request; `Ready` means the agent is signed out.
    ///
    /// # Errors
    ///
    /// [`LauncherError`] when this connection was not sent the request, or the call fails.
    pub async fn report_logout(
        &self,
        request: &LoginRequestId,
        outcome: LoginOutcome,
    ) -> Result<(), LauncherError> {
        let (word, reason) = outcome.to_wire();
        self.bus
            .launcher_proxy()
            .await?
            .report_agent_logout(request.as_str(), word, reason)
            .await
            .map_err(|e| launcher_error(&e))
    }
}
