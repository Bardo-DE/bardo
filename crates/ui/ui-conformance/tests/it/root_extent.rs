//! A root holding only a Sheet is 0 px tall unless it asks for the
//! viewport. `RootExtent::Viewport` makes it the viewport's size, measured on a real Blitz
//! document. The sheet's overlay is placed against the viewport either way, so the sheet stands
//! in the same place in both; what the 0 px root loses is its own box (its ground, its hits).

use crate::support::probe::length;
use dioxus::prelude::*;
use std::time::Duration;
use ui_components::components::chrome::split_view::view::SplitView;
use ui_components::prelude::*;
use ui_harness::{Clock, Harness, HarnessConfig, Viewport};

const VIEW: Viewport = Viewport {
    width: 640,
    height: 400,
    scale_percent: 100,
};

/// An overlay root, `extent` as asked, holding one small sheet.
#[component]
fn Overlay(extent: RootExtent) -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Sheet, extent,
            Sheet { label: "Power", onclose: move |()| {},
                p { "Shut down?" }
            }
        }
    }
}

#[expect(
    non_snake_case,
    reason = "a Dioxus component function is named in CamelCase"
)]
fn Content() -> Element {
    rsx! { Overlay { extent: RootExtent::Content } }
}

#[expect(
    non_snake_case,
    reason = "a Dioxus component function is named in CamelCase"
)]
fn Full() -> Element {
    rsx! { Overlay { extent: RootExtent::Viewport } }
}

fn laid_out(app: fn() -> Element) -> Harness {
    let mut harness = Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(600));
    harness
}

#[test]
fn a_content_root_of_only_a_sheet_is_zero_tall_while_its_overlay_covers_the_viewport() {
    let harness = laid_out(Content);
    let root = harness.rect(".ds").expect("the root is laid out");
    assert_eq!(root.size.height.0, 0.0, "the trap Q94 names: {root:?}");
    let overlay = harness
        .rect(".ds-overlay")
        .expect("the overlay is laid out");
    assert_eq!(
        (overlay.size.width.0, overlay.size.height.0),
        (length(VIEW.width), length(VIEW.height)),
        "{overlay:?}"
    );
    let sheet = |harness: &Harness| harness.rect(".ds-sheet").expect("the sheet is laid out");
    assert_eq!(
        sheet(&harness),
        sheet(&laid_out(Full)),
        "placed as in a viewport root"
    );
}

#[test]
fn a_viewport_root_is_the_viewports_size_and_its_sheet_is_placed_in_it() {
    let harness = laid_out(Full);
    assert_eq!(
        harness.attr(".ds", "data-extent").as_deref(),
        Some("viewport")
    );
    let root = harness.rect(".ds").expect("the root is laid out");
    assert_eq!(root.size.height.0, length(VIEW.height), "{root:?}");
    assert_eq!(root.size.width.0, length(VIEW.width), "{root:?}");
    let sheet = harness.rect(".ds-sheet").expect("the sheet is laid out");
    assert!(
        sheet.size.height.0 > 20.0,
        "the sheet has its content's height: {sheet:?}"
    );
    assert!(
        sheet.origin.y.0 + sheet.size.height.0 <= length(VIEW.height),
        "the sheet is inside the viewport: {sheet:?}"
    );
}

#[expect(
    non_snake_case,
    reason = "a Dioxus component function is named in CamelCase"
)]
fn Window() -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window, extent: RootExtent::Window,
            SplitView { label: "Panes", panes: Vec::new(), p { "Only a short line" } }
        }
    }
}

#[test]
fn a_window_root_fills_the_window_so_a_full_height_child_needs_no_container() {
    let harness = laid_out(Window);
    let root = harness.rect(".ds").expect("the root is laid out");
    assert_eq!(root.size.height.0, length(VIEW.height), "{root:?}");
    assert_eq!(root.size.width.0, length(VIEW.width), "{root:?}");
    let split = harness.rect(".ds-split").expect("the split is laid out");
    assert_eq!(split.size.height.0, length(VIEW.height), "{split:?}");
}
