//! The mail-app states (controls): each new prop or variant rendered through dioxus-ssr and
//! linted as markup. Every state here is additive; the states that existed before are in
//! `components_controls.rs`, `components_lists.rs` and `button_field_tile_states_ssr.rs`.

#[path = "provider_and_button_face_states/cases.rs"]
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
fn every_provider_and_button_face_state_lints_clean() {
    let failures: Vec<String> = CASES
        .iter()
        .filter_map(|case| specimen::lint(case.name, &render(case.make)).err())
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
