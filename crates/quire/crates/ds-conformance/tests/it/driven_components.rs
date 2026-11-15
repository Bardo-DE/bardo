//! Components driven through a real Blitz document: pointer, keys and time.
//!
//! The root stamps the last input modality; a toggle switches; a hover card appears after 500 ms
//! and not before; a toast hides after its hold, both driven through the hubs the root provides;
//! a field focused on mount takes typing; and the Space editor reports the dot picked inside it
//! and does not pick one when a stop chip is removed. The menu, the row roster and the
//! button-click rules have their own files.

use dioxus::prelude::*;
use ds::components::app::space_editor::DotIndex;
use ds::components::overlays::hover_card::target::HoverTarget;
use ds::prelude::*;
use ds::stack::hover_hub::{HoverKey, HoverKind, use_hover_hub};
use ds::style::space::presets::PRESETS;
use ds_harness::harness::settle_until;
use ds_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 480,
    height: 360,
    scale_percent: 100,
};

/// Wraps a test's content in the root every quire surface draws inside.
#[component]
fn Root(children: Element) -> Element {
    rsx! {
        Ds { sheet: Some(ds_shell::stylesheet()), appearance: Appearance::default(), material: Material::Window, {children} }
    }
}

fn flip(switch: Check) -> Check {
    switch.flipped()
}

/// The centre of `selector`, which the test expects to be on screen.
fn centre(harness: &Harness, selector: &str) -> Point {
    harness
        .centre(selector)
        .unwrap_or_else(|| panic!("{selector} is not in the document:\n{}", harness.html()))
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

// ---- Controls ----------------------------------------------------------------------------

#[allow(non_snake_case)]
fn PressApp() -> Element {
    let mut pinned = use_signal(|| Check::Off);
    rsx! {
        Root {
            Button {
                label: "Pin",
                value: Some(pinned()),
                onclick: move |_| pinned.set(flip(pinned())),
            }
        }
    }
}

#[test]
fn the_root_stamps_the_last_input_modality() {
    let mut harness = Harness::new(
        PressApp,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    let modality = |harness: &Harness| harness.attr(".ds", "data-modality");
    assert_eq!(modality(&harness).as_deref(), Some("pointer"));
    harness.send(Input::key(ShortcutKey::Tab));
    assert_eq!(modality(&harness).as_deref(), Some("keyboard"));
    harness.send(Input::click(centre(&harness, ".ds-button")));
    assert_eq!(modality(&harness).as_deref(), Some("pointer"));
}

#[allow(non_snake_case)]
fn ToggleApp() -> Element {
    let mut wifi = use_signal(|| Check::Off);
    rsx! {
        Root {
            Toggle { label: "Wi-Fi", value: wifi(), onchange: move |next| wifi.set(next) }
        }
    }
}

#[test]
fn a_toggle_switches() {
    let mut harness = Harness::new(
        ToggleApp,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    let checked = |harness: &Harness| harness.attr(".ds-toggle", "aria-checked");
    assert_eq!(checked(&harness).as_deref(), Some("false"));
    harness.send(Input::click(centre(&harness, ".ds-toggle")));
    assert_eq!(checked(&harness).as_deref(), Some("true"));
    harness.send(Input::click(centre(&harness, ".ds-toggle")));
    assert_eq!(checked(&harness).as_deref(), Some("false"));
}

// ---- Component cases (overlays and lists) ---------------------------------------------------

#[allow(non_snake_case)]
fn HoverCardApp() -> Element {
    rsx! {
        Root { HoverCardDemo {} }
    }
}

#[allow(non_snake_case)]
fn HoverCardDemo() -> Element {
    let hub = use_hover_hub();
    rsx! {
        HoverTarget { hover_key: HoverKey("sender:3".into()), kind: HoverKind::Sender,
            span { "Dana Okafor" }
        }
        if hub.open().is_some() {
            HoverCard { kind: HoverKind::Sender, p { "dana@example.com" } }
        }
    }
}

#[test]
fn a_hover_card_appears_after_500_ms_and_not_before() {
    let mut harness = Harness::new(
        HoverCardApp,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    let rested = harness.now();
    harness.send(Input::pointer_move(centre(&harness, ".ds-hover-target")));
    // Under half the 500 ms open delay, not near it (fixed 2026-09-25, FINDINGS "Timing tests"):
    // the old check left only 50 ms of margin (11 % of the window).
    harness.advance(ms(200));
    assert_eq!(harness.count(".ds-hovercard"), 0, "open well before 500 ms");
    let opened = settle_until(&mut harness, |h| h.count(".ds-hovercard") == 1);
    assert_eq!(
        opened.duration_since(rested),
        ms(500),
        "appeared only once the full delay had run"
    );
}

#[allow(non_snake_case)]
fn ToastApp() -> Element {
    rsx! {
        Root { ToastDemo {} }
    }
}

#[allow(non_snake_case)]
fn ToastDemo() -> Element {
    let toasts = use_toasts();
    rsx! {
        Button {
            label: "Archive",
            onclick: move |_| toasts.push("Archived".into(), None),
        }
    }
}

#[test]
fn a_toast_hides_after_its_hold() {
    let mut harness = Harness::new(
        ToastApp,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    let shown = |harness: &Harness| harness.attr(".ds-toast", "data-presence");
    let pushed = harness.now();
    harness.send(Input::click(centre(&harness, ".ds-button")));
    // It arrives sliding in from the right, and is present once that has settled.
    assert_eq!(shown(&harness).as_deref(), Some("entering"));
    harness.advance(ms(400));
    assert_eq!(shown(&harness).as_deref(), Some("present"));
    assert_eq!(
        harness.text_of(".ds-toast-body").as_deref(),
        Some("Archived")
    );
    // Half the 5000 ms hold, not all of it (FINDINGS "Timing tests"): a check at the deadline
    // leaves no margin against a loaded machine's overshoot on `advance`.
    harness.advance(ms(2500));
    assert_eq!(
        shown(&harness).as_deref(),
        Some("present"),
        "hid well before the hold ends"
    );
    let hidden = settle_until(&mut harness, |h| shown(h).as_deref() == Some("leaving"));
    assert_eq!(
        hidden.duration_since(pushed),
        ms(5000),
        "hid only once the full hold had run"
    );
    // Slid out: after `--t-quick` and a frame nothing is laid out.
    harness.advance(ms(500));
    assert_eq!(harness.count(".ds-toast"), 0);
}

// ---- Wave 2 integration props ------------------------------------------------------------

#[allow(non_snake_case)]
fn FocusApp() -> Element {
    rsx! {
        Root {
            Field { focus: FieldFocus::OnMount }
        }
    }
}

#[allow(non_snake_case)]
fn ManualApp() -> Element {
    rsx! {
        Root {
            Field { focus: FieldFocus::Manual }
        }
    }
}

#[component]
fn Field(focus: FieldFocus) -> Element {
    let mut text = use_signal(String::new);
    rsx! {
        TextField {
            label: "Link",
            value: text(),
            focus,
            oninput: move |next| text.set(next),
        }
        p { class: "probe-text", "[{text}]" }
    }
}

#[test]
fn a_field_focused_on_mount_takes_typing_without_a_click() {
    /// An app, and what its field shows after one key.
    type Case = (fn() -> Element, &'static str);
    const CASES: &[Case] = &[(FocusApp, "[a]"), (ManualApp, "[]")];
    for &(app, want) in CASES {
        let mut harness = Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
        harness.advance(ms(100));
        assert_eq!(harness.text_of(".probe-text").as_deref(), Some("[]"));
        harness.send(Input::key(ShortcutKey::Char('a')));
        assert_eq!(
            harness.text_of(".probe-text").as_deref(),
            Some(want),
            "{}",
            harness.html()
        );
    }
}

#[allow(non_snake_case)]
fn EditorApp() -> Element {
    let mut active = use_signal(|| DotIndex(0));
    let mut look = use_signal(|| SpaceLook {
        dots: PRESETS[0].dots.to_vec(),
        grain: Grain(35),
        theme: Theme::System,
    });
    rsx! {
        Root {
            SpaceEditor {
                look: look(),
                scheme: Scheme::Light,
                active_dot: active(),
                name: "Work".to_string(),
                onchange: move |next| look.set(next),
                on_active_dot: move |dot| active.set(dot),
            }
            p { class: "probe-dot", "{active().0}" }
        }
    }
}

#[test]
fn the_space_editor_reports_the_dot_picked_inside_it() {
    let mut harness = Harness::new(
        EditorApp,
        HarnessConfig::new(Viewport {
            height: 900,
            ..VIEW
        })
        .with_clock(Clock::Virtual),
    );
    assert_eq!(harness.text_of(".probe-dot").as_deref(), Some("0"));
    assert!(
        harness.count(".ds-stops .ds-chip") > 1,
        "{}",
        harness.html()
    );
    harness.send(Input::click(centre(
        &harness,
        ".ds-stops .ds-chip:nth-child(2) .ds-chip-press",
    )));
    assert_eq!(harness.text_of(".probe-dot").as_deref(), Some("1"));
    assert_eq!(
        harness
            .attr(
                ".ds-stops .ds-chip:nth-child(2) .ds-chip-press",
                "aria-pressed"
            )
            .as_deref(),
        Some("true")
    );
}

#[test]
fn removing_a_stop_chip_removes_it_and_does_not_pick_it() {
    let mut harness = Harness::new(
        EditorApp,
        HarnessConfig::new(Viewport {
            height: 900,
            ..VIEW
        })
        .with_clock(Clock::Virtual),
    );
    assert_eq!(harness.count(".ds-stops .ds-chip"), 2);
    let remove = ".ds-stops .ds-chip:nth-child(2) .ds-chip-remove";
    harness.send(Input::click(centre(&harness, remove)));
    assert_eq!(harness.count(".ds-stops .ds-chip"), 1);
    assert_eq!(harness.text_of(".probe-dot").as_deref(), Some("0"));
}
