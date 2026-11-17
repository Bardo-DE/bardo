//! The roles a caller can have: the one vocabulary of the desktop's daemons.
//!
//! A daemon learns who is calling from the caller table (`account-callers`, over peephole's
//! judgement of the process) and answers with a [`CallerRole`]. What each role may ask a daemon for
//! is that daemon's own matter: each keeps its permission predicates over these roles instead of
//! mapping them into an enum of its own.

use serde::{Deserialize, Serialize};

/// What a caller may ask for, beyond what its consent allows.
///
/// It grows as the desktop gains
/// processes with a standing of their own, so a match outside porter needs a `_` arm (an
/// unknown role is best treated as a plain app).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallerRole {
    /// Any identified app.
    App,
    /// detent: may use `SettingsModule1` (remove, toggle, revoke, client ids).
    Settings,
    /// sill, or a standalone sheet host: may serve `AccountsSheet1`. memoryd and inferd take the
    /// one named `org.bardo.Shell` for the shell.
    SheetHost,
    /// syncd, as the peer of the desktop's daemons. accountd has no interface for it beyond the
    /// computers on the person's tailnet: it keeps identities, and nothing a daemon asks for on
    /// an app's behalf.
    PorterDaemon,
    /// inferd, the AI broker: the one daemon that sends a key to an AI company, so the one that
    /// may ask accountd (`org.bardo.Accounts1.Broker`) what each app's permits say and for the
    /// key behind a permit. It also reads the person's computers from `org.bardo.Tailnet1`, as
    /// the porter daemons do. Given by the unit that is inferd, never by an app scope's name.
    AiBroker,
    /// intentd, companiond, readerd, cuad, bardo-do, bardo-actions-mcp: refused accountd's `Keys`
    /// (`RequestKey`) and `AddAccount` and `Reauthenticate`; they keep inferd's own inference
    /// calls. memoryd takes the one named `org.bardo.Intents` for the action router, and inferd
    /// the ones its table lists under `places` for those that may choose where the assistant runs.
    Agent,
    /// cuad, for inferd: the only role that may open a computer-use session.
    Cua,
    /// The agent launcher (bardo-agent-acp), which runs external coding agents: may call accountd's
    /// `Launcher` interface (`RegisterLauncher`, `BeginSession`, `EndSession`, `RequestAgentKey`,
    /// `ReportAgentLogin`, `ReportAgentLogout`, `IssueProcessCredential` and
    /// `RevokeProcessCredential`: an API key for a process it spawns, only for a program it
    /// registered) and inferd's `Agents` interface, and nothing else. Never granted by default:
    /// the shipped tables have no row for it, so a machine lists the launcher by its own row, in
    /// accountd's table and in inferd's.
    AgentLauncher,
    /// The terminal (temor): may read the person's computers from `org.bardo.Tailnet1`
    /// (`Machines`) and nothing else of accountd. Given by the unit that is the terminal, as the
    /// systemd manager says its main process is, never by an app scope's name.
    Terminal,
    /// The compositor (casement): mints a desktop-wide Space id for each workspace and
    /// registers it with `org.bardo.Spaces1` (`Register`); it may `List` the Spaces and nothing
    /// else of accountd. Given by the compositor's unit, never by an app scope's name.
    Compositor,
}

impl CallerRole {
    /// Whether the role stands for more than a plain app, so only the system's table may give
    /// it: `Settings`, `SheetHost`, `PorterDaemon`, `AiBroker`, `AgentLauncher`, `Cua`, `Terminal` and
    /// `Compositor`. `App` and `Agent` hold no standing beyond an app (`Agent` is refused more
    /// than `App` is), so a user's file may name them.
    #[must_use]
    pub const fn is_fixed(self) -> bool {
        !matches!(self, Self::App | Self::Agent)
    }
}

#[cfg(test)]
mod tests {
    use super::CallerRole;

    #[test]
    fn only_app_and_agent_hold_no_standing() {
        assert!(!CallerRole::App.is_fixed());
        assert!(!CallerRole::Agent.is_fixed());
        for role in [
            CallerRole::Settings,
            CallerRole::SheetHost,
            CallerRole::PorterDaemon,
            CallerRole::AiBroker,
            CallerRole::Cua,
            CallerRole::AgentLauncher,
            CallerRole::Terminal,
            CallerRole::Compositor,
        ] {
            assert!(role.is_fixed(), "{role:?}");
        }
    }
}
