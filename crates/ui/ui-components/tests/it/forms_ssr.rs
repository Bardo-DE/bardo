//! Form, `FormSection`, `IconTile` and `PaneStack` in every state rendered through dioxus-ssr and
//! linted as markup; every class they draw is styled by the components' sheets.

#[path = "forms/cases.rs"]
mod cases;
use crate::css_scan;
use crate::support::scoped;
use crate::support::specimen;

use cases::{CASES, Case};
use css_scan::{classes, styles_class};
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

fn render(case: &Case) -> String {
    let mut dom = VirtualDom::new_with_props(host, HostProps { make: case.make });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn case_name(case: &Case) -> String {
    format!("forms/{}/{}", case.component, case.state)
}

/// The sheets of the forms components and of what a state draws inside them.
const SHEETS: &[&str] = &[
    "form",
    "form_section",
    "icon_tile",
    "pane_stack",
    "button",
    "list",
    "row",
    "avatar",
];

fn sheet(name: &str) -> &'static str {
    ui_components::component_sheets()
        .iter()
        .find(|(n, _)| *n == name)
        .map_or("", |(_, css)| *css)
}

#[test]
fn every_forms_state_lints_clean() {
    let failures: Vec<String> = CASES
        .iter()
        .filter_map(|case| specimen::lint(&case_name(case), &render(case)).err())
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_class_in_a_forms_state_is_styled() {
    let mut failures = Vec::new();
    for case in CASES {
        let html = render(case);
        for class in classes(&html) {
            let styled = class == "ds-ic"
                || class == "ds-truncate"
                || SHEETS.iter().any(|n| styles_class(sheet(n), class));
            if class.starts_with("ds-") && !styled {
                failures.push(format!(
                    "{}: .{class} is styled by no sheet",
                    case_name(case)
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Reduced motion drops the slide and keeps the fade: the sheet's own rule, since the spring that
/// drives `--pane-q` is the same at every level.
#[test]
fn the_pane_stack_does_not_slide_under_reduced_motion() {
    let rule = ".ds[*|data-motion=reduced] .ds-pane-stack-page{ transform:none; }";
    assert!(sheet("pane_stack").contains(rule), "missing {rule}");
}
