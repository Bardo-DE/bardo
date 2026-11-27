//! The control components (design/04-COMPONENTS.md sections 1-7, 9-15): every component in
//! every state rendered through dioxus-ssr and linted as markup, plus scans that every `ds-`
//! class a state renders is styled by that component's CSS and that the CSS uses tokens only.

mod cases;
mod cases_catalogue;
use crate::css_scan;
use crate::support::scoped;
use crate::support::specimen;

use cases::{CASES, Case, MOTION_CASES};
use css_scan::{STYLES, classes, styles_class, token_violations};
use dioxus::prelude::*;
use scoped::Scoped;
use ui_components::prelude::*;
use ui_style::icon::render::Glyph;
use ui_style::icon::shape::Shape;

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
/// that reads the scope (a text field) draws instead of panicking.
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

/// Renders every case, lints it as markup, and checks that every `ds-` class it draws is styled
/// by the sheets of its own component.
fn check_all(cases: &[Case]) {
    let mut failures = Vec::new();
    for case in cases {
        let name = format!("controls/{}/{}", case.component, case.state);
        let html = render(case.make);
        failures.extend(specimen::lint(&name, &html).err());
        // A field that failed to draw renders as nothing; every text-field state must hold it.
        if case.component == "text_field" && !html.contains("class=\"ds-text-field") {
            failures.push(format!("{name}: no text field drawn"));
        }
        let Some((_, sheets)) = STYLES.iter().find(|(c, _)| *c == case.component) else {
            failures.push(format!(
                "{name}: no stylesheet listed for {}",
                case.component
            ));
            continue;
        };
        for class in classes(&html) {
            // `ds-ic` is `Glyph`'s: sized by its own attributes, never by component CSS (S6).
            // `a-*` pulse classes come from the motion stylesheet.
            if class == "ds-ic" || !class.starts_with("ds-") {
                continue;
            }
            if !sheets.iter().any(|css| styles_class(css, class)) {
                failures.push(format!(
                    "{name}: .{class} is in no stylesheet of {}",
                    case.component
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_control_renders_clean() {
    check_all(CASES);
}

#[test]
fn every_catalogue_control_renders_clean() {
    check_all(cases_catalogue::CASES);
}

#[test]
fn motion_driven_controls_render_clean() {
    check_all(MOTION_CASES);
}

#[test]
fn the_token_scan_catches_what_it_bans() {
    const BAD: &[&str] = &[
        ".x{ color:#fff; }",
        ".x{ color:rgb(0,0,0); }",
        ".x[data-variant=a]{}",
        ".x{ transition:color 170ms linear; }",
        ".x{ animation:spin 1.1s linear; }",
        ".x{ font-family:Karla; }",
        ".x:focus-visible{}",
    ];
    for css in BAD {
        assert!(!token_violations(css).is_empty(), "missed: {css}");
    }
    assert_eq!(
        token_violations(".x[*|data-variant=a]{ font-size:var(--fs-note); padding:1.5px 7px; }"), [] as [std::string::String; 0]
    );
}

#[test]
fn a_glyph_is_stroked_and_sized_by_its_attributes() {
    const SIZES: &[(IconSize, &str)] = &[
        (IconSize::Micro, "11"),
        (IconSize::Compact, "14"),
        (IconSize::Base, "16"),
        (IconSize::Bar, "22"),
    ];
    for (size, px) in SIZES {
        #[derive(Props, Clone, PartialEq)]
        struct At {
            size: IconSize,
        }
        #[expect(
            clippy::needless_pass_by_value,
            reason = "a Dioxus component takes its props by value"
        )]
        fn glyph(props: At) -> Element {
            rsx! { Glyph { icon: Icon::Wifi, size: props.size, style: GlyphStyle::Outline } }
        }
        let mut dom = VirtualDom::new_with_props(glyph, At { size: *size });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        for want in [
            "class=\"ds-ic\"".to_string(),
            format!("width=\"{px}\""),
            format!("height=\"{px}\""),
            format!("data-size=\"{px}\""),
            "stroke=\"currentColor\"".to_string(),
            "stroke-width=\"2\"".to_string(),
            "stroke-linecap=\"round\"".to_string(),
            "stroke-linejoin=\"round\"".to_string(),
            "fill=\"none\"".to_string(),
        ] {
            assert!(html.contains(&want), "{size:?}: no {want} in {html}");
        }
    }
}

#[test]
fn every_shell_glyph_has_geometry() {
    let empty: Vec<String> = Icon::SHELL
        .iter()
        .filter(|icon| icon.shapes().is_empty())
        .map(|icon| format!("{icon:?}"))
        .collect();
    assert!(empty.is_empty(), "no shapes: {}", empty.join(", "));
    // Lucide 1.47.0 `wifi`, first path; Tabler `brightness-half` without its bounding path.
    assert_eq!(
        Icon::Wifi.shapes().first(),
        Some(&Shape::Path("M12 20h.01"))
    );
    assert_eq!(
        Icon::Brightness.shapes().first(),
        Some(&Shape::Path("M12 9a3 3 0 0 0 0 6v-6"))
    );
}

#[test]
fn a_shortcut_is_glyphs_without_separators() {
    let shortcut = Shortcut(vec![
        ShortcutKey::Ctrl,
        ShortcutKey::Shift,
        ShortcutKey::Char('t'),
    ]);
    assert_eq!(shortcut.glyphs(), "⌃⇧T");
    assert_eq!(
        Shortcut(vec![
            ShortcutKey::Super,
            ShortcutKey::Alt,
            ShortcutKey::Enter
        ])
        .glyphs(),
        "⌥⌘↵",
        "the Mac's order, whatever order the keys were given in"
    );
    assert_eq!(Shortcut::default().glyphs(), "");
}

/// The slider consumes wheel input itself (11-BEHAVIOUR-scroll.md section 11.3.1 item 2:
/// "sliders, zoomable canvases"): layer-shell-host's scroll engine hands `data-wheel="capture"`
/// elements the raw `BlitzWheelEvent` instead of scrolling the page under them. Every slider
/// case carries the marker, not just the default one, so a state that later drops it (a variant,
/// a disabled slider) is caught here rather than only by a look at the markup.
#[test]
fn every_slider_carries_the_wheel_capture_marker() {
    let sliders: Vec<&Case> = CASES
        .iter()
        .chain(MOTION_CASES.iter())
        .filter(|case| case.component == "slider")
        .collect();
    assert!(!sliders.is_empty(), "no slider cases to check");
    for case in sliders {
        let html = render(case.make);
        assert!(
            html.contains("data-wheel=\"capture\""),
            "{}/{} is missing data-wheel=\"capture\": {html}",
            case.component,
            case.state
        );
    }
}
