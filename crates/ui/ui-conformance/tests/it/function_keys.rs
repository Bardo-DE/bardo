//! A function key sent through the harness reaches an app as chordkit's action: F9 is
//! `ToggleSidebar` in the CUA table, and a Mac table (Ctrl+Command+S) leaves it alone.

use dioxus::prelude::*;
use keymap::{Action, Context, Desktop, Platform, StandardAction};
use std::fmt::Write as _;
use std::time::Duration;
use ui_blitz::FocusFallback;
use ui_components::prelude::*;
use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};

const VIEW: Viewport = Viewport {
    width: 300,
    height: 120,
    scale_percent: 100,
};

#[expect(
    non_snake_case,
    reason = "a Dioxus component function is named in CamelCase"
)]
fn Page() -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window, extent: RootExtent::Viewport,
            Pad {}
        }
    }
}

/// Under `Ds`, so `use_keys` reads the root's keymap (the harness's platform), not the default.
#[allow(
    non_snake_case,
    reason = "a Dioxus component function is named in CamelCase"
)]
#[component]
fn Pad() -> Element {
    let keys = ui_components::keys::use_keys();
    let mut log = use_signal(String::new);
    rsx! {
        div {
            id: "pad",
            tabindex: "0",
            style: "width:200px;height:60px",
            onkeydown: on_action(keys, Context::Normal, move |action| {
                log.with_mut(|log| write!(log, "{action:?};").expect("writing to a String cannot fail"));
                true
            }),
            "pad"
        }
        span { id: "log", "{log}" }
    }
}

fn started(platform: Platform) -> Harness {
    let config = HarnessConfig::new(VIEW)
        .with_clock(Clock::Virtual)
        .with_focus_fallback(FocusFallback::BlitzDefault)
        .with_platform(platform);
    let mut harness = Harness::new(Page, config);
    harness.advance(Duration::from_millis(100));
    // Tab gives the pad the keys (a click does not focus a plain element here).
    harness.send(Input::key(ShortcutKey::Tab));
    harness.advance(Duration::from_millis(16));
    harness
}

fn heard(harness: &Harness) -> String {
    harness.text_of("#log").unwrap_or_default()
}

#[test]
fn f9_is_toggle_sidebar_under_the_cua_table() {
    let mut harness = started(Platform::Linux {
        desktop: Desktop::Other,
    });
    harness.send(Input::key(ShortcutKey::F(9)));
    harness.advance(Duration::from_millis(16));
    harness.advance(Duration::from_millis(16));
    let want = format!("{:?};", Action::Standard(StandardAction::ToggleSidebar));
    assert_eq!(heard(&harness), want);
}

#[test]
fn f9_means_nothing_to_the_mac_table() {
    let mut harness = started(Platform::MacOs);
    harness.send(Input::key(ShortcutKey::F(9)));
    harness.advance(Duration::from_millis(16));
    harness.advance(Duration::from_millis(16));
    assert_eq!(heard(&harness), "");
}
