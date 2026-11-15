//! The dock's glyph tiles in the dark scheme, read off the painted pixels: after a live switch to
//! Dark the Trash and the window glyph kept their light-scheme ink, dark on the dark dock.

use std::time::Duration;

use sill_dock::Dock;
use sill_ui_kit::harness::{SurfaceHarness, assert_inks_read_in_each_scheme, fake};
use sill_ui_kit::hooks::PinnedAppearance;

/// The glyph tiles: the Trash, and the window glyph an app without an icon gets.
const INKS: &[(&str, &str)] = &[
    (
        "the Trash glyph",
        ".ds-dock-tile[*|aria-label=Trash] svg.ds-ic",
    ),
    ("the first window glyph", ".ds-dock-tile svg.ds-ic"),
];

#[test]
fn a_dock_reads_in_either_scheme_and_after_a_live_switch() {
    assert_inks_read_in_each_scheme(INKS, |environment| {
        let mut run = SurfaceHarness::<Dock>::setup((), ())
            .appearance(PinnedAppearance(environment))
            .start();
        run.feeds.apps.send_replace(fake::dock::index());
        run.feeds.pins.send_replace(fake::dock::pins());
        run.feeds.badges.send_replace(fake::dock::badges());
        run.feeds
            .windows
            .send_modify(|wl| wl.toplevels = fake::dock::toplevels_on("DP-1"));
        run.harness.advance(Duration::from_secs(1));
        run
    });
}
