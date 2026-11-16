//! The Displays page over a compositor that answers from memory: the page without a compositor, and
//! a display's rows. The changes the page sends are tested in `settings-app-model`.

use crate::support::Sandbox;
use crate::support::drive::{page_of, showing, sidebar_page, title_of};
use settings_app_model::{
    DisplayMode, DisplayPosition, DisplayRotation, DisplaysStatus, Millimetres, Monitor, OutputId,
    PixelSize, RefreshMilliHz, ScaleMilli,
};
use settings_app_system::displays::FakeDisplays;
use settings_app_ui::Opening;
use ui_harness::Harness;

const PAGE: &str = "[data-system=\"displays\"]";

fn displays() -> Opening {
    Opening {
        page: Some(settings_app_model::PageSlug("displays".to_owned())),
        query: String::new(),
    }
}

fn mode(width: u32, height: u32, hertz: u32, preferred: bool) -> DisplayMode {
    DisplayMode {
        size: PixelSize { width, height },
        refresh: RefreshMilliHz(hertz * 1000),
        preferred,
    }
}

fn id(name: &str) -> OutputId {
    OutputId(name.to_owned())
}

/// A 14 inch panel: 3024 by 1964 at 120 or 60 Hertz, looking like 1512 by 982.
fn laptop() -> Monitor {
    let native = mode(3024, 1964, 120, true);
    Monitor {
        id: id("eDP-1"),
        name: "Studio Panel".to_owned(),
        modes: vec![native, mode(3024, 1964, 60, false)],
        current: Some(native),
        scale: ScaleMilli(2000),
        rotation: DisplayRotation::Standard,
        position: DisplayPosition { x: 0, y: 0 },
        size_mm: Some(Millimetres {
            width: 302,
            height: 189,
        }),
    }
}

fn world(name: &str, service: &FakeDisplays) -> (Sandbox, Harness) {
    let sandbox = Sandbox::new(name).with_displays(service.clone());
    let harness = sandbox.harness(displays());
    (sandbox, harness)
}

fn row(display: &str, nth: usize) -> String {
    format!("[data-display=\"{display}\"] .ds-field-row:nth-child({nth})")
}

fn one() -> DisplaysStatus {
    DisplaysStatus {
        displays: vec![laptop()],
    }
}

#[test]
fn the_page_sits_after_desktop_and_dock_and_says_plainly_when_there_is_no_compositor() {
    let sandbox = Sandbox::new("displays-absent");
    let mut harness = sandbox.harness(displays());
    showing(
        &mut harness,
        "the unavailable row",
        "[data-system=\"unavailable\"]",
    );
    assert_eq!(page_of(&harness).as_deref(), Some("displays"));
    assert_eq!(title_of(&harness).as_deref(), Some("Displays"));
    assert_eq!(
        harness.text_of(&sidebar_page("dock")).as_deref(),
        Some("Desktop & Dock")
    );
    assert_eq!(
        harness.text_of(&sidebar_page("displays")).as_deref(),
        Some("Displays")
    );
    let row = harness
        .text_of("[data-system=\"unavailable\"]")
        .unwrap_or_default();
    assert!(
        row.contains("Displays aren\u{2019}t available right now."),
        "{row}"
    );
}

#[test]
fn a_display_shows_its_name_and_its_resolution_refresh_rate_and_rotation() {
    let service = FakeDisplays::answering(one());
    let (_sandbox, mut harness) = world("displays-one", &service);
    showing(&mut harness, "the page", PAGE);
    showing(&mut harness, "the display", "[data-display=\"eDP-1\"]");
    assert!(harness.html().contains("Studio Panel"));
    let resolution = harness.text_of(&row("eDP-1", 1)).unwrap_or_default();
    assert!(resolution.contains("Resolution"), "{resolution}");
    assert!(
        resolution.contains("Default for display (1512 \u{d7} 982)"),
        "the default is the scale that suits the panel's density: {resolution}"
    );
    assert!(
        resolution.contains("Looks like 1512 \u{d7} 982"),
        "{resolution}"
    );
    let refresh = harness.text_of(&row("eDP-1", 2)).unwrap_or_default();
    assert!(
        refresh.contains("Refresh Rate") && refresh.contains("120 Hertz"),
        "{refresh}"
    );
    let rotation = harness.text_of(&row("eDP-1", 3)).unwrap_or_default();
    assert!(
        rotation.contains("Rotation") && rotation.contains("Standard"),
        "{rotation}"
    );
    assert_eq!(
        harness.count("[data-display=\"eDP-1\"] .ds-field-row"),
        4,
        "resolution, refresh rate, rotation and mode size; no Position row with one display"
    );
}
