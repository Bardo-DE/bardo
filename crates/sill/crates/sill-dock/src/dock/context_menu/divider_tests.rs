//! Table tests for the dock's own menu.

use ds::prelude::*;

use super::rows;
use crate::dock::context_menu::{DockMenuRow, menu_entries};
use crate::dock::pick::DockPick;

fn listed(autohide: bool, magnification: bool) -> Vec<(String, Option<DockPick>)> {
    menu_entries(&rows(autohide, magnification), None, &[])
        .into_iter()
        .map(|e| match e {
            MenuItem::Item { title, value, .. } => (title, Some(value)),
            _ => ("--".to_owned(), None),
        })
        .collect()
}

#[test]
fn the_rows_are_hiding_magnification_a_rule_and_dock_settings() {
    assert_eq!(
        rows(false, true),
        [
            DockMenuRow::Hiding(false),
            DockMenuRow::Magnifying(true),
            DockMenuRow::Separator,
            DockMenuRow::DockSettings,
        ]
    );
}

#[test]
fn the_toggles_read_by_the_state_they_will_set_and_dock_settings_opens_the_page() {
    let cases = [
        (false, true, "Turn Hiding On", "Turn Magnification Off"),
        (true, false, "Turn Hiding Off", "Turn Magnification On"),
    ];
    for (hiding, magnifying, want_hiding, want_magnifying) in cases {
        let menu = listed(hiding, magnifying);
        assert_eq!(
            menu,
            [
                (want_hiding.to_owned(), Some(DockPick::ToggleHiding)),
                (
                    want_magnifying.to_owned(),
                    Some(DockPick::ToggleMagnification)
                ),
                ("--".to_owned(), None),
                (
                    "Dock Settings…".to_owned(),
                    Some(DockPick::OpenDockSettings)
                ),
            ]
        );
    }
}
