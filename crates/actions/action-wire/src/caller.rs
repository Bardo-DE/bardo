//! Who is calling the router, as the transport derived it. Roles ([`CallerRole`], in
//! `action-vocab`) come from `actiond.toml` and the connection's identity; they are never sent
//! in a body.

use account_core::AppId;
use action_vocab::CallerRole;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Scope name prefixes of a terminal's children: a VTE terminal's spawned shell, tmux's server
/// and a login session (tty or ssh).
const TERMINAL_SCOPES: [&str; 3] = ["vte-spawn-", "tmux-spawn-", "session-"];

/// Whether the cgroup path (the unified line of `/proc/<pid>/cgroup`) ends in a terminal child's
/// scope.
///
/// The one place the desktop says what a terminal is: actiond derives the cli role from it and
/// companiond lets the same processes open, ask and close.
#[must_use]
pub fn is_terminal_scope(cgroup: &str) -> bool {
    let leaf = cgroup.rsplit('/').next().unwrap_or_default();
    #[expect(
        clippy::case_sensitive_file_extension_comparisons,
        reason = "a systemd unit name is matched exactly: `.scope` is a unit type suffix, not a file extension"
    )]
    let is_scope = leaf.ends_with(".scope");
    is_scope && TERMINAL_SCOPES.iter().any(|p| leaf.starts_with(p))
}

/// A caller as the transport derived it: an identity and the roles `actiond.toml` gives that bus
/// name.
///
/// One name may play several (the shell is the launcher, the confirm server and the control centre); the
/// role a call acts in is the first that may make it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallerId {
    /// The application, as the accounts service derives it from the connection.
    pub app: AppId,
    /// The roles of the name; empty means a plain app.
    pub roles: BTreeSet<CallerRole>,
}
