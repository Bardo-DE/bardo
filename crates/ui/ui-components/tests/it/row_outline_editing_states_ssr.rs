//! The mail-app states: a tree item renamed in place (its editing slot in the label's place)
//! rendered through dioxus-ssr and linted as markup. The trailing slot's `data-slot="trailing"`
//! shows in these and in the mail-app tree states.

mod cases;
use crate::support::scoped;
use crate::support::specimen;

use cases::CASES;
use dioxus::prelude::*;

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

/// Renders a case inside a scope, so its handlers have a runtime to attach to.
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Dioxus component takes its props by value"
)]
fn host(props: HostProps) -> Element {
    (props.make)()
}

fn render(make: fn() -> Element) -> String {
    let mut dom = VirtualDom::new_with_props(host, HostProps { make });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn every_row_outline_editing_state_lints_clean() {
    let failures: Vec<String> = CASES
        .iter()
        .filter_map(|case| specimen::lint(case.name, &render(case.make)).err())
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
