//! The role of this crate's service in the shipped caller table (`packaging/dist/desktop-callers.toml`).

use caller_identity_core::RoleName;

/// The compositor (`compositor`).
pub const COMPOSITOR: RoleName = RoleName::from_static("compositor");

#[cfg(test)]
mod tests {
    use super::*;
    use caller_identity_core::Table;

    const SHIPPED: &str = include_str!("../../../../packaging/dist/desktop-callers.toml");

    #[test]
    fn the_shipped_table_has_a_row_for_each_role_here() {
        let table = Table::from_toml(SHIPPED).expect("the shipped table reads");
        assert!(
            table.has_role(&COMPOSITOR),
            "the shipped table has no row for {COMPOSITOR}"
        );
    }
}
