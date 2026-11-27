//! The shipped sample table reads, and its units and programs are as expected.

use caller_identity_core::Table;

const SHIPPED: &str = include_str!("../../../dist/desktop-callers.toml");

#[test]
fn the_shipped_table_reads() {
    let table = Table::from_toml(SHIPPED).expect("dist/desktop-callers.toml reads");
    assert_ne!(table.rows(), []);
}

#[test]
fn units_are_services_and_unitless_rows_are_programs() {
    let table = Table::from_toml(SHIPPED).expect("the shipped table reads");
    let program_roles: Vec<&str> = table
        .rows()
        .iter()
        .filter(|r| r.unit.is_none())
        .map(|r| r.role.as_str())
        .collect();
    assert_eq!(
        program_roles,
        ["updater_cli", "sandbox_launcher", "manager"]
    );
}
