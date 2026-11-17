//! `actiond-callers.toml`: which unit is which app, and with what role.
//!
//! The shipped file is `dist/actiond-callers.toml`; the system's copy under `/etc/bardo` and the
//! person's under their configuration directory are laid over each other
//! (`action_dbus::callers::read_table`), and only the system's can give a fixed role.

use action_dbus::callers::{
    CallerFileError, CallerTable, read_table, read_table_at, table_from_toml,
};
use std::path::Path;

/// The file name in both directories.
pub const FILE: &str = "actiond-callers.toml";

const SHIPPED: &str = include_str!("../../dist/actiond-callers.toml");

/// The table the package ships. A shipped file that does not read is an empty table: nobody is
/// identified, which is the strict way to fail.
#[must_use]
pub fn shipped() -> CallerTable {
    table_from_toml(SHIPPED).unwrap_or_default()
}

/// The table of this machine: `/etc/bardo/actiond-callers.toml` with the person's file over it.
///
/// # Errors
///
/// Returns a [`CallerFileError`] when a callers file exists and cannot be used.
pub fn read(env: &impl Fn(&str) -> Option<String>) -> Result<CallerTable, CallerFileError> {
    read_table(FILE, env)
}

/// As [`read`], with the system's file at `system` (the acceptance harness's own system).
///
/// # Errors
///
/// Returns a [`CallerFileError`] when a callers file exists and cannot be used.
pub fn read_at(
    system: &Path,
    env: &impl Fn(&str) -> Option<String>,
) -> Result<CallerTable, CallerFileError> {
    read_table_at(system, FILE, env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use caller_role_types::CallerRole;

    #[test]
    fn the_shipped_file_reads_and_gives_the_shell_the_sheet_host_role_by_its_unit() {
        let table = shipped();
        let shell = table
            .resolve_unit("bardo-shell.service")
            .expect("the shell's unit");
        assert_eq!(shell.app.name.as_str(), "org.bardo.Shell");
        assert_eq!(shell.role, CallerRole::SheetHost);
        assert!(table.resolve_unit("bardo-shell.scope").is_none());
    }
}
