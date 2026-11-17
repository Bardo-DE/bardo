//! The peers the desktop judges a bus message's sender with.

use caller_identity_dbus::caller_identity_core::{Proof, Requirement, Table, roles};
use caller_identity_dbus::caller_identity_linux::{RealProc, TableFiles};
use caller_identity_dbus::{BusPeers, ManagerCheck};
use zbus::Connection;

/// Peers on `connection`, judging against the desktop's caller table (the root-owned files the caller-identity library
/// loads) and believing the unit manager only when its own owner is the system's.
///
/// A table that cannot be read leaves an empty one, so no program is a service and every question
/// is refused: Settings then shows its services as unavailable instead of trusting a name.
///
/// A build with `dev-callers` adds the rows of the person's own file (`bardo-settings --dev-callers`) after
/// the table; a release build has no such rows.
#[must_use]
pub fn desktop_peers(connection: &Connection) -> BusPeers {
    let table = TableFiles::desktop()
        .load(&RealProc::system())
        .unwrap_or_else(|why| {
            eprintln!("bardo-settings: the list of the desktop's own programs could not be read ({why}); no program is trusted");
            Table::empty()
        });
    #[cfg(feature = "dev-callers")]
    let table = crate::dev::with_dev_rows(table);
    let manager = ManagerCheck::Verified(Requirement::new(roles::MANAGER, Proof::Exe));
    BusPeers::new(connection.clone(), table, manager)
}
