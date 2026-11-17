//! The mail-app states (controls and tiles) and 3 (the muted avatar): each new prop or variant
//! rendered through dioxus-ssr and linted as markup. Every state here is additive; the states
//! that existed before are in `components_controls.rs` and `components_lists.rs`.

mod cases;
use crate::support::scoped;
use crate::support::specimen;

use cases::CASES;
use dioxus::prelude::*;
use scoped::Scoped;

#[derive(Props, Clone)]
struct HostProps {
    make: fn() -> Element,
}

/// Never equal: the host renders once, and function addresses are not comparable anyway.
impl PartialEq for HostProps {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

/// Renders a case inside a scope, so its handlers have a runtime to attach to and a component
/// that reads the scope (a text field) draws instead of panicking, which dioxus-ssr renders as
/// nothing.
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Dioxus component takes its props by value"
)]
fn host(props: HostProps) -> Element {
    let case = (props.make)();
    rsx! { Scoped { {case} } }
}

fn render(make: fn() -> Element) -> String {
    let mut dom = VirtualDom::new_with_props(host, HostProps { make });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn every_button_field_and_tile_state_lints_clean() {
    let failures: Vec<String> = CASES
        .iter()
        .filter_map(|case| specimen::lint(case.name, &render(case.make)).err())
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A field that failed to draw renders as nothing: every text-field state must hold the field.
#[test]
fn every_text_field_state_draws_the_field() {
    let blank: Vec<&str> = CASES
        .iter()
        .filter(|case| case.name.starts_with("controls/text_field/"))
        .filter(|case| !render(case.make).contains("class=\"ds-text-field"))
        .map(|case| case.name)
        .collect();
    assert!(blank.is_empty(), "states with no text field: {blank:?}");
}
