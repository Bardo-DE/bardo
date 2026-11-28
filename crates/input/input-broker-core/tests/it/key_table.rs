//! The key table's own consistency.

use input_broker_core::keytable::{KEY_TABLE, class_of};

#[test]
fn the_table_has_no_duplicate_names_or_codes() {
    for (i, a) in KEY_TABLE.iter().enumerate() {
        for b in &KEY_TABLE[i + 1..] {
            assert!(
                a.name != b.name && a.code != b.code,
                "{} and {}",
                a.name,
                b.name
            );
        }
    }
}

#[test]
fn the_four_modifiers_are_in_the_table() {
    for modifier in [29, 42, 56, 125] {
        assert!(class_of(modifier).is_some(), "modifier {modifier}");
    }
}
