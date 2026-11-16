//! The bar's glyphs and text in the dark scheme, read off the painted pixels: switching to Dark
//! from the control center (a live change, not a start in Dark) left the status icons and the
//! workspace pills in their light-scheme colours, dark on a dark bar.

use std::time::Duration;

use shell_bar::Bar;
use shell_ui_kit::harness::{SurfaceHarness, assert_inks_read_in_each_scheme};
use shell_ui_kit::hooks::PinnedAppearance;
use shell_ui_kit::links::ControlCenterLink;

/// What the bar draws that must read: the status items, the workspace pills.
const INKS: &[(&str, &str)] = &[
    ("the volume glyph", "[*|aria-label=Volume]"),
    ("the Wi-Fi glyph", "[*|aria-label=Network]"),
    ("the battery glyph", "[*|aria-label=Battery]"),
    ("the control center item", "[*|aria-label='Control Center']"),
    ("the workspace pills", ".ds-ws-pills"),
];

#[test]
fn a_bar_reads_in_either_scheme_and_after_a_live_switch() {
    assert_inks_read_in_each_scheme(INKS, |environment| {
        let mut run = SurfaceHarness::<Bar>::setup(ControlCenterLink::new(), ())
            .appearance(PinnedAppearance(environment))
            .start();
        run.harness.advance(Duration::from_secs(1));
        run
    });
}
