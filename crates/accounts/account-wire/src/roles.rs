//! The roles of the services this crate names in the shipped caller table (`packaging/dist/desktop-callers.toml`).

use caller_identity_core::RoleName;

/// The accounts service (`bardo-accountd`).
pub const ACCOUNTS: RoleName = RoleName::from_static("accounts");
/// The model-access service (`bardo-inferd`).
pub const INFERENCE: RoleName = RoleName::from_static("inference");

#[cfg(test)]
mod tests {
    use super::*;
    use caller_identity_core::Table;

    const SHIPPED: &str = include_str!("../../../../packaging/dist/desktop-callers.toml");

    #[test]
    fn the_shipped_table_has_a_row_for_each_role_here() {
        let table = Table::from_toml(SHIPPED).expect("the shipped table reads");
        for role in [ACCOUNTS, INFERENCE] {
            assert!(
                table.has_role(&role),
                "the shipped table has no row for {role}"
            );
        }
    }
}
