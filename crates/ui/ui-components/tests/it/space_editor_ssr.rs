//! The Space editor and the Space dot: every state rendered through dioxus-ssr; the field plane
//! decoded and probed, the handles placed from the dots, the contrast pills against
//! `space::readout`; and every state's classes styled, its sheet on tokens, and its markup
//! linted against the stylesheet the design system draws with.

use crate::css_scan;
#[path = "lists/editor.rs"]
mod field_plane;
#[path = "lists/png.rs"]
mod png;
#[path = "lists/space_editor_rows.rs"]
mod space_editor_rows;

use css_scan::{classes, styles_class};
use dioxus::prelude::*;
use ui_components::components::app::space_editor::DotIndex;
use ui_components::components::app::space_editor::rows::EditorFrame;
use ui_components::prelude::*;
use ui_style::space::palette::readout::readout;
use ui_style::space::palette::{Capping, Dot, derive, swatch};
use ui_style::space::presets::PRESETS;

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

/// Preset `index` as a Space's look.
fn preset_look(index: usize, grain: Grain) -> SpaceLook {
    SpaceLook {
        dots: PRESETS[index].dots.to_vec(),
        grain,
        theme: Theme::System,
    }
}

fn editor(look: SpaceLook, scheme: Scheme, active: u8) -> Element {
    rsx! { SpaceEditor { look, scheme, active_dot: DotIndex(active), onchange: |_| {} } }
}

/// One state.
struct Case {
    state: &'static str,
    make: fn() -> Element,
}

const CASES: &[Case] = &[
    // SpaceEditor: the two sample Spaces in each scheme and a one-dot Space.
    Case {
        state: "work-light",
        make: || editor(preset_look(0, Grain(35)), Scheme::Light, 0),
    },
    Case {
        state: "work-dark",
        make: || editor(preset_look(0, Grain(35)), Scheme::Dark, 1),
    },
    Case {
        state: "home-light",
        make: || editor(preset_look(1, Grain(55)), Scheme::Light, 2),
    },
    Case {
        state: "one-dot",
        make: || editor(preset_look(2, Grain(40)), Scheme::Light, 0),
    },
    Case {
        state: "named",
        make: || rsx! { SpaceEditor { look: preset_look(0, Grain(35)), scheme: Scheme::Light, active_dot: DotIndex(0), name: "Work".to_string(), onchange: |_| {}, on_active_dot: |_| {} } },
    },
    Case {
        state: "frameless",
        make: || rsx! { SpaceEditor { look: preset_look(0, Grain(35)), scheme: Scheme::Light, active_dot: DotIndex(0), onchange: |_| {}, frame: EditorFrame::Frameless } },
    },
    // SpaceColour: the colour half alone, unframed for a popover.
    Case {
        state: "colour",
        make: || rsx! { SpaceColour { look: preset_look(0, Grain(35)), scheme: Scheme::Light, active_dot: DotIndex(1), onchange: |_| {} } },
    },
    // SpaceDot.
    Case {
        state: "space-dot-current",
        make: || rsx! { SpaceDot { name: "Work", frame: ui_style::space::frame_vars::FrameVars::of(&preset_look(0, Grain(35)), Scheme::Light), selection: true, shortcut: Shortcut(vec![ShortcutKey::Ctrl, ShortcutKey::Char('1')]), onclick: |()| {} } },
    },
    Case {
        state: "space-dot-elsewhere",
        make: || rsx! { SpaceDot { name: "Home", frame: ui_style::space::frame_vars::FrameVars::of(&preset_look(1, Grain(55)), Scheme::Dark), selection: false, shortcut: Shortcut(vec![ShortcutKey::Ctrl, ShortcutKey::Char('2')]), onclick: |()| {} } },
    },
    Case {
        state: "space-dot-foot-current",
        make: || rsx! { SpaceDot { name: "Work", face: DotFace::Foot, frame: ui_style::space::frame_vars::FrameVars::of(&preset_look(0, Grain(35)), Scheme::Light), selection: true, shortcut: Shortcut(vec![ShortcutKey::Ctrl, ShortcutKey::Char('1')]), onclick: |()| {} } },
    },
    Case {
        state: "space-dot-foot-elsewhere",
        make: || rsx! { SpaceDot { name: "Home", face: DotFace::Foot, frame: ui_style::space::frame_vars::FrameVars::of(&preset_look(1, Grain(55)), Scheme::Dark), selection: false, shortcut: Shortcut(vec![ShortcutKey::Ctrl, ShortcutKey::Char('2')]), onclick: |()| {} } },
    },
];

/// Every state rendered, by name.
fn rendered() -> Vec<(String, String)> {
    CASES
        .iter()
        .map(|case| (format!("space_editor/{}", case.state), render(case.make)))
        .chain(
            space_editor_rows::CASES
                .iter()
                .map(|case| (case.name.to_string(), render(case.make))),
        )
        .collect()
}

/// The stylesheets that style the editor's markup: its own, then those of the components it
/// renders.
fn sheets() -> Vec<&'static str> {
    let all = ui_components::component_sheets();
    [
        "space_editor",
        "section_header",
        "segmented",
        "slider",
        "button",
        "chip",
        "text_field",
        "label",
        "progress",
    ]
    .iter()
    .filter_map(|name| all.iter().find(|(n, _)| n == name).map(|(_, css)| *css))
    .collect()
}

/// Classes every component may use that its own sheets do not define: `Glyph`'s `ds-ic` (sized
/// by its attributes, spike S6) and the `.ds-truncate` utility.
const SHARED: &[&str] = &["ds-ic", "ds-truncate"];

#[test]
fn every_class_in_a_state_is_styled_by_the_editors_sheets() {
    let css = sheets();
    assert_eq!(css.len(), 9, "a sheet the editor draws with is missing");
    let mut failures = Vec::new();
    for (name, html) in &rendered() {
        for class in classes(html) {
            if SHARED.contains(&class) || !class.starts_with("ds-") {
                continue;
            }
            if !css.iter().any(|sheet| styles_class(sheet, class)) {
                failures.push(format!(
                    "{name}: .{class} is in no stylesheet of the editor"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// What the editor writes inline that a consumer may not: colours computed per Space. Each is
/// data the palette computes, not a theme colour.
const MARKUP_EXCEPTIONS: &[ui_lint::Exception] = {
    use ui_lint::{Exception, Rule};
    &[
        Exception {
            rule: Rule::HexColour,
            selector: "div.ds-handle",
            reason: "a handle is filled with its dot's pick colour from space::palette",
        },
        Exception {
            rule: Rule::HexColour,
            selector: "i.ds-chip-dot",
            reason: "a stop chip's dot is its colour's pick from space::palette",
        },
        Exception {
            rule: Rule::HexColour,
            selector: "span.ds-space-swatch",
            reason: "the title swatch is the Space's gradient from space::palette",
        },
        Exception {
            rule: Rule::HexColour,
            selector: "button.ds-preset",
            reason: "a preset is painted with its own derived gradient",
        },
        Exception {
            rule: Rule::HexColour,
            selector: "button.ds-space-dot",
            reason: "a Space dot is painted with its FrameVars gradient",
        },
    ]
};

/// Coherence rule 2 on the editor's output: every state uses only classes the stylesheet
/// styles, no hand-written SVG or form control, and no colour inline beyond the exceptions.
#[test]
fn every_space_editor_state_lints_clean() {
    use ui_lint::{LintConfig, markup};
    let config = LintConfig {
        exceptions: MARKUP_EXCEPTIONS,
        ..LintConfig::new(&ui_components::kits())
    };
    let failures: Vec<String> = rendered()
        .iter()
        .flat_map(|(name, html)| {
            markup(html, ui_components::stylesheet(), &config)
                .into_iter()
                .map(move |offence| format!("{name}: {:?} {}", offence.rule, offence.text))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
