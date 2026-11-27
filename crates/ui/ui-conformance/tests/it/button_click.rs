//! Where `Harness::click` reaches a Button (FINDINGS "Polish pass: the inline-context click").
//!
//! A Button is `display:inline-flex`, an atomic inline. A block after the Button (the anchor
//! test's caption) or a flex parent puts the Button in a box of its own and the click lands. (A
//! Button alone in an inline formatting context is not clicked: blitz-dom at the pinned rev hits
//! the parent, so that case has no test until the pin is fixed.)

use dioxus::prelude::*;
use std::time::Duration;
use ui_components::prelude::*;
use ui_harness::{Clock, Harness, HarnessConfig, Input, Viewport};

const VIEW: Viewport = Viewport {
    width: 480,
    height: 360,
    scale_percent: 100,
};

/// A Button that shows whether it was clicked, and nothing after it but an `if` placeholder.
#[component]
fn Clickable() -> Element {
    let mut state = use_signal(|| Check::Off);
    rsx! {
        Button {
            label: format!("{:?}", state()),
            onclick: move |_| state.set(Check::On),
        }
        if state() == Check::On {
            span { "Opened" }
        }
    }
}

#[expect(
    non_snake_case,
    reason = "a Dioxus component function is named in CamelCase"
)]
fn BlockAfterApp() -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window,
            div { style: "padding:40px", Clickable {} p { "after it, a block" } }
        }
    }
}

#[expect(
    non_snake_case,
    reason = "a Dioxus component function is named in CamelCase"
)]
fn FlexApp() -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window,
            div { style: "padding:40px; display:flex", Clickable {} }
        }
    }
}

/// Click the Button's centre and read its label: `On` once the click reached it.
fn label_after_click(app: fn() -> Element) -> String {
    let mut harness = Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    let at = harness
        .centre(".ds-button")
        .expect("the button is on screen");
    harness.send(Input::click(at));
    harness.advance(Duration::from_millis(50));
    harness.text_of(".ds-button").unwrap_or_default()
}

#[test]
fn a_click_reaches_a_button_followed_by_a_block() {
    assert_eq!(label_after_click(BlockAfterApp), "On");
}

#[test]
fn a_click_reaches_a_button_in_a_flex_row_with_only_a_placeholder_after_it() {
    assert_eq!(label_after_click(FlexApp), "On");
}
