//! Which program sent a bus message, the way every desktop service asks it: hold the sender,
//! judge it against a caller table, and believe the unit manager's answers about its unit only
//! when the manager is the system's own.
//!
//! The services of the desktop (the portal, the updater, the shell's bus objects, the compositor,
//! Settings, the accounts service) each need the same three things, and this crate is the one
//! place that has them: the desktop's own table read from its root-owned files ([`desktop_table`]),
//! the requirement the unit manager must meet ([`manager_need`]), and the bus peers built over a
//! table ([`peers_on`]). What a service then accepts from a caller is its own
//! [`caller_identity_core::Requirement`]s.

use caller_identity_core::{Proof, Requirement, RoleName, Table};
use caller_identity_dbus::{BusPeers, ManagerCheck};
use caller_identity_linux::{LoadError, RealProc, TableFiles};
use zbus::Connection;

/// The unit manager (`systemd --user`, which runs the system's `systemd`): the role whose answers
/// about units are believed. A test in this crate checks it against the shipped table.
pub const MANAGER: RoleName = RoleName::from_static("manager");

/// What the owner of the unit manager's bus name must be before its answers about units are
/// believed: the manager's row, at the proof an image alone gives.
#[must_use]
pub fn manager_need() -> Requirement {
    Requirement::new(MANAGER, Proof::Exe)
}

/// The desktop's table of its own programs, read from the root-owned files it ships in
/// (`/etc/bardo/desktop-callers.toml` and the files beside it).
///
/// # Errors
///
/// Returns the [`LoadError`] when a file is missing, is not root-only or is not a table. A service
/// then runs with [`Table::empty`], where nobody is a service and every call is refused, and says
/// why in its own log.
pub fn desktop_table() -> Result<Table, LoadError> {
    TableFiles::desktop().load(&RealProc::system())
}

/// Bus peers on `connection`, judging against `table` and believing the unit manager only when
/// its own owner meets [`manager_need`].
#[must_use]
pub fn peers_on(connection: Connection, table: Table) -> BusPeers {
    BusPeers::new(connection, table, ManagerCheck::Verified(manager_need()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHIPPED: &str = include_str!("../../../../packaging/dist/desktop-callers.toml");

    #[test]
    fn the_shipped_table_has_a_row_for_the_manager() {
        let table = Table::from_toml(SHIPPED).expect("the shipped table reads");
        assert!(table.has_role(&MANAGER));
    }

    #[test]
    fn the_manager_is_asked_for_at_the_proof_of_its_image() {
        let need = manager_need();
        assert_eq!(need.role, MANAGER);
        assert_eq!(need.at_least, Proof::Exe);
    }
}
