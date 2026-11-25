//! `agent-acp-callers.toml`.
//!
//! The one process bardo-agent-acp and bardo-agent answer when they serve `Confirm1` is actiond's
//! unit.
//!
//! The shipped file is `dist/agent-acp-callers.toml`; the system's copy
//! under `/etc/bardo` and the person's under their configuration directory are laid over each
//! other (`action_dbus::callers::read_table`), and only the system's can give a fixed role.

use action_dbus::callers::{
    CallerFileError, CallerTable, read_table, read_table_at, system_override, table_from_toml,
};
use std::path::{Path, PathBuf};

/// The file name in both directories.
pub const FILE: &str = "agent-acp-callers.toml";

/// The variable that names the file to read as the system's table, in place of
/// `/etc/bardo/agent-acp-callers.toml`: a build with the `test-system-callers` feature only.
pub const SYSTEM_CALLERS_VAR: &str = "BARDO_AGENT_ACP_SYSTEM_CALLERS";

/// Whether this build honours [`SYSTEM_CALLERS_VAR`]: only with the `test-system-callers` feature,
/// so that no test reads this computer's `/etc/bardo`. A shipped build ignores the variable.
pub const HONOURS_SYSTEM_CALLERS: bool = cfg!(feature = "test-system-callers");

/// The system's table a test build was told to read, when the variable is set (a shipped build
/// ignores it and says so on standard error).
#[must_use]
pub fn system_file(env: &impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    system_override(
        HONOURS_SYSTEM_CALLERS,
        "bardo-agent-acp",
        SYSTEM_CALLERS_VAR,
        env,
    )
}

const SHIPPED: &str = include_str!("../../dist/agent-acp-callers.toml");

/// The table the package ships. A shipped file that does not read is an empty table: nobody is
/// identified, which is the strict way to fail.
#[must_use]
pub fn shipped() -> CallerTable {
    table_from_toml(SHIPPED).unwrap_or_default()
}

/// The table of this machine: `/etc/bardo/agent-acp-callers.toml` with the person's file over it.
///
/// # Errors
///
/// [`CallerFileError`] when the system's or the person's callers file cannot be read or does not
/// parse.
pub fn read(env: &impl Fn(&str) -> Option<String>) -> Result<CallerTable, CallerFileError> {
    match system_file(env) {
        Some(system) => read_table_at(&system, FILE, env),
        None => read_table(FILE, env),
    }
}

/// As [`read`], with the system's file at `system` (the acceptance harness's own system).
///
/// # Errors
///
/// [`CallerFileError`] when the system's or the person's callers file cannot be read or does not
/// parse.
pub fn read_at(
    system: &Path,
    env: &impl Fn(&str) -> Option<String>,
) -> Result<CallerTable, CallerFileError> {
    read_table_at(system, FILE, env)
}
