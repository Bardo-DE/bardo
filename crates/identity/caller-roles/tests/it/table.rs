//! The role constants are rows of the shipped caller table.

use caller_identity_core::Table;
use caller_roles::{AGENT_TIER, PORTAL, PORTAL_FRONTEND, SETTINGS, SHELL};

const SHIPPED: &str = include_str!("../../../../../packaging/dist/desktop-callers.toml");

#[test]
fn the_shipped_table_has_a_row_for_each_role() {
    let table = Table::from_toml(SHIPPED).expect("the shipped table reads");
    for role in [SHELL, PORTAL, PORTAL_FRONTEND, AGENT_TIER, SETTINGS] {
        assert!(
            table.has_role(&role),
            "the shipped table has no row for {role}"
        );
    }
}
