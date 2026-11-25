//! `actiond-callers.toml`: which unit is which app, and with what role.
//!
//! The shipped file is `dist/actiond-callers.toml`; the system's copy under `/etc/bardo` and the
//! person's under their configuration directory are laid over each other
//! (`action_dbus::callers::read_table_at`), and only the system's can give a fixed role. Which
//! system file is read is an input of the setup ([`crate::SystemFiles`]), so a test gives its own.

use action_dbus::callers::{CallerFileError, CallerTable, read_table_at, table_from_toml};
use std::path::{Path, PathBuf};

/// The file name in both directories.
pub const FILE: &str = "actiond-callers.toml";

/// The text of the file the package ships.
pub const SHIPPED: &str = include_str!("../../dist/actiond-callers.toml");

/// The variable that names the file to read as the system's table, in place of
/// `/etc/bardo/actiond-callers.toml`: a build with the `test-system-callers` feature only.
pub const SYSTEM_CALLERS_VAR: &str = "BARDO_ACTIOND_SYSTEM_CALLERS";

/// Whether this build honours [`SYSTEM_CALLERS_VAR`]: only with the `test-system-callers` feature,
/// so that no test reads this computer's `/etc/bardo`. A shipped build ignores the variable.
pub const HONOURS_SYSTEM_CALLERS: bool = cfg!(feature = "test-system-callers");

/// The system's table a test build was told to read, when the variable is set (a shipped build
/// ignores it and says so on standard error).
#[must_use]
pub fn system_override(env: &impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    action_dbus::callers::system_override(
        HONOURS_SYSTEM_CALLERS,
        "actiond",
        SYSTEM_CALLERS_VAR,
        env,
    )
}

/// The table the package ships. A shipped file that does not read is an empty table: nobody is
/// identified, which is the strict way to fail.
#[must_use]
pub fn shipped() -> CallerTable {
    table_from_toml(SHIPPED).unwrap_or_default()
}

/// The table of this machine: the system's file at `system` (`/etc/bardo/actiond-callers.toml`
/// when installed) with the person's file over it. A system file that is missing is an empty
/// table.
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
