//! `agent-acp-callers.toml`.
//!
//! The one process bardo-agent-acp and bardo-agent answer when they serve `Confirm1` is intentd's
//! unit.
//!
//! The shipped file is `dist/agent-acp-callers.toml`; the system's copy
//! under `/etc/bardo` and the person's under their configuration directory are laid over each
//! other (`action_dbus::callers::read_table`), and only the system's can give a fixed role.

use action_dbus::callers::{
    CallerFileError, CallerTable, read_table, read_table_at, table_from_toml,
};
use std::path::Path;

/// The file name in both directories.
pub const FILE: &str = "agent-acp-callers.toml";

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
    read_table(FILE, env)
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
