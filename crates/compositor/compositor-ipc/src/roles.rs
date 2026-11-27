//! The role of this crate's service in the shipped caller table (`dist/desktop-callers.toml` of the identity folder).

use caller_identity_core::RoleName;

/// A sandbox launcher (`flatpak`): may create security contexts.
pub const SANDBOX_LAUNCHER: RoleName = RoleName::from_static("sandbox_launcher");

#[cfg(test)]
mod tests {
    use super::*;
    use caller_identity_core::Table;

    const SHIPPED: &str = include_str!("../../../identity/dist/desktop-callers.toml");

    #[test]
    fn the_shipped_table_has_a_row_for_each_role_here() {
        let table = Table::from_toml(SHIPPED).expect("the shipped table reads");
        assert!(
            table.has_role(&SANDBOX_LAUNCHER),
            "the shipped table has no row for {SANDBOX_LAUNCHER}"
        );
    }
}
