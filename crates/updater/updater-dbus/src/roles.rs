//! The roles of the services this crate names in the shipped caller table (`dist/desktop-callers.toml` of the identity folder).

use caller_identity_core::RoleName;

/// The software-update service (`bardo-updater.service`).
pub const UPDATER: RoleName = RoleName::from_static("updater");
/// The `updater` command line: a program, not a service.
pub const UPDATER_CLI: RoleName = RoleName::from_static("updater_cli");

#[cfg(test)]
mod tests {
    use super::*;
    use caller_identity_core::Table;

    const SHIPPED: &str = include_str!("../../../identity/dist/desktop-callers.toml");

    #[test]
    fn the_shipped_table_has_a_row_for_each_role_here() {
        let table = Table::from_toml(SHIPPED).expect("the shipped table reads");
        for role in [UPDATER, UPDATER_CLI] {
            assert!(
                table.has_role(&role),
                "the shipped table has no row for {role}"
            );
        }
    }
}
