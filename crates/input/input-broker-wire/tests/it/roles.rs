//! The role constants of the wire crate are rows of the shipped caller table.

use caller_identity_core::Table;
use input_broker_wire::{KEYMAP_DAEMON, PAD_DAEMON};

const SHIPPED: &str = include_str!("../../../../identity/dist/desktop-callers.toml");

#[test]
fn the_shipped_table_has_a_row_for_each_daemon_role() {
    let table = Table::from_toml(SHIPPED).expect("the shipped table reads");
    for role in [PAD_DAEMON, KEYMAP_DAEMON] {
        assert!(
            table.has_role(&role),
            "the shipped table has no row for {role}"
        );
    }
}
