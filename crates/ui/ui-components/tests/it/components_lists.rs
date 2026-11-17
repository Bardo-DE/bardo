//! The list, sidebar and Space-frame components (design/04-COMPONENTS.md sections 8, 16, 17, 19,
//! 26-28, 32-35): every component in every state rendered through dioxus-ssr and
//! linted as markup and checked against its component's sheets; roster-driven lists at each
//! moment of a row's exit; the drag ghost driven by a `DragTracker`; the Space editor's field
//! plane decoded and probed; and the contrast pills against `space::readout`.

#[path = "lists/cases.rs"]
mod cases;
use crate::css_scan;
use crate::support::dom_time;
use crate::support::specimen;
#[path = "lists/live.rs"]
mod live;
#[path = "lists/motion.rs"]
mod motion;
#[path = "lists/rows.rs"]
mod rows;
use crate::support::scoped;
#[path = "lists/strip_press.rs"]
mod strip_press;
#[path = "lists/thread_row.rs"]
mod thread_row;

use cases::CASES;
use css_scan::{classes, styles_class};
use dioxus::prelude::*;
use rows::ROW_CASES;
use strip_press::STRIP_PRESS_CASES;
use thread_row::THREAD_ROW_CASES;
use ui_motion::drag::{DragPhase, DragTracker, use_drag};
use ui_motion::presence::Exit;

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

/// The stylesheets that style one component's markup: its own, then those of the components it
/// renders.
fn sheets(component: &str) -> Vec<&'static str> {
    let own: &[&str] = match component {
        "thread_row" | "roster" => &[
            "thread_row",
            "row",
            "list",
            "hover_strip",
            "row_more",
            "button",
            "provider_mark",
            "chip",
            "text_runs",
        ],
        "row" => &[
            "row",
            "disclosure",
            "badge",
            "avatar",
            "button",
            "icon_view",
            "icon_tile",
            "toggle",
            "progress",
            "key_equivalent",
            "status_glyph",
            "text_field",
            "text_runs",
        ],
        "list" => &[
            "list",
            "row",
            "thread_row",
            "hover_strip",
            "button",
            "provider_mark",
            "chip",
            "section_header",
            "icon_tile",
            "avatar",
            "disclosure",
            "text_runs",
        ],
        "pin_tile" => &["pin_tile", "avatar", "provider_mark", "badge"],
        "hover_strip" => &["hover_strip", "icon_button"],
        "row_more" => &["row_more"],
        "drag" => &["drag_ghost"],
        other => return sheet(other).into_iter().collect(),
    };
    own.iter().filter_map(|name| sheet(name)).collect()
}

/// The stylesheet of the component called `name`.
fn sheet(name: &str) -> Option<&'static str> {
    ui_components::component_sheets()
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, css)| *css)
}

/// Classes every component may use that its own sheets do not define: `Glyph`'s `ds-ic` (sized
/// by its attributes, spike S6) and the `.ds-truncate` utility.
const SHARED: &[&str] = &["ds-ic", "ds-truncate"];

/// A class a component here draws that another component's sheet styles: the picker's Space dot
/// is the `space_editor` sheet's (FINDINGS "The picker's Space dot").
const STYLED_ABOVE: &[&str] = &["ds-space-dot"];

#[test]
fn every_list_component_renders_clean_and_is_styled_by_its_component() {
    let mut failures = Vec::new();
    for case in CASES
        .iter()
        .chain(ROW_CASES)
        .chain(THREAD_ROW_CASES)
        .chain(STRIP_PRESS_CASES)
    {
        let name = format!("lists/{}/{}", case.component, case.state);
        let html = render(case.make);
        failures.extend(specimen::lint(&name, &html).err());
        let css = sheets(case.component);
        if css.is_empty() {
            failures.push(format!(
                "{name}: no stylesheet listed for {}",
                case.component
            ));
            continue;
        }
        for class in classes(&html) {
            if SHARED.contains(&class) || STYLED_ABOVE.contains(&class) || !class.starts_with("ds-")
            {
                continue;
            }
            if !css.iter().any(|sheet| styles_class(sheet, class)) {
                failures.push(format!(
                    "{name}: .{class} is in no stylesheet of {}",
                    case.component
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
