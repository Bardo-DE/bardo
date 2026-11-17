//! Why accountd refuses a launcher's call, beside the refusals of `reply::Refusal`. `NoLauncher` is a
//! `Refusal` (the sheet's answer to a login nobody can do); these are the launcher's own. On the
//! bus each is an error named `<service's error prefix>.<name>`, which `account-dbus` reads.

/// Why accountd refused a launcher's call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LauncherWireError {
    /// A live connection already launches this program (first wins).
    AlreadyRegistered,
    /// No such request for this connection: never made, answered already, expired, or another
    /// connection's.
    UnknownRequest,
    /// `IssueProcessCredential` for a program this connection has no live registration for
    /// (`RegisterLauncher`): another connection's, or nobody's.
    NotRegistered,
    /// `IssueProcessCredential` under a `Once` permit: it is spent by its first use, and a second
    /// turn of the agent would find the key gone.
    OnceGrant,
    /// `IssueProcessCredential` under a grant of an account that holds no API key to hand over
    /// (an OAuth account, an agent that signs itself in, a local runtime).
    NotAKeyAccount,
    /// `RevokeProcessCredential` for a credential this connection was not issued: never made,
    /// ended already, or another connection's.
    UnknownCredential,
    /// `BeginSession` for a session id another live connection holds (first wins).
    SessionTaken,
    /// `EndSession`, `RequestAgentKey` or `IssueProcessCredential` for a session that is not
    /// open for this connection: never begun, ended already, or another connection's.
    UnknownSession,
}

impl std::fmt::Display for LauncherWireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            LauncherWireError::AlreadyRegistered => "another launcher already holds a program",
            LauncherWireError::UnknownRequest => "no such request for this connection",
            LauncherWireError::NotRegistered => "this connection has not registered that program",
            LauncherWireError::OnceGrant => "a once permit cannot back a process credential",
            LauncherWireError::NotAKeyAccount => "the account holds no API key to hand over",
            LauncherWireError::UnknownCredential => "no such credential for this connection",
            LauncherWireError::SessionTaken => "another launcher holds that session",
            LauncherWireError::UnknownSession => "no such open session for this connection",
        })
    }
}

impl std::error::Error for LauncherWireError {}

impl LauncherWireError {
    /// Every fault, so the table is total by construction.
    pub const ALL: [LauncherWireError; 8] = [
        LauncherWireError::AlreadyRegistered,
        LauncherWireError::UnknownRequest,
        LauncherWireError::NotRegistered,
        LauncherWireError::OnceGrant,
        LauncherWireError::NotAKeyAccount,
        LauncherWireError::UnknownCredential,
        LauncherWireError::SessionTaken,
        LauncherWireError::UnknownSession,
    ];

    /// The bare name of the fault, the last part of its bus error name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            LauncherWireError::AlreadyRegistered => "AlreadyRegistered",
            LauncherWireError::UnknownRequest => "UnknownRequest",
            LauncherWireError::NotRegistered => "NotRegistered",
            LauncherWireError::OnceGrant => "OnceGrant",
            LauncherWireError::NotAKeyAccount => "NotAKeyAccount",
            LauncherWireError::UnknownCredential => "UnknownCredential",
            LauncherWireError::SessionTaken => "SessionTaken",
            LauncherWireError::UnknownSession => "UnknownSession",
        }
    }

    /// The fault a bare name stands for.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|fault| fault.name() == name)
    }
}
