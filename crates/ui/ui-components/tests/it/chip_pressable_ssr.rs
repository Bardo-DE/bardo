//! A chip's pressable form, dot and remove as markup: a chip without `onpress` stays a static
//! label, one with it draws a `button` carrying `aria-pressed`, a dot is a disc coloured by its
//! own property, and a remove button follows the text on any variant.

use dioxus::core::NoOpMutations;
use dioxus::prelude::*;
use ui_components::components::controls::chip::{Chip, ChipVariant};
use ui_components::components::controls::chip_dot::ChipDot;
use ui_components::prelude::*;
use ui_style::tokens::label_hue::LabelHue;
use ui_style::tokens::system_hue::SystemHue;

fn render(app: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);
    let page = dioxus_ssr::render(&dom);
    let from = page.find("<span class=\"ds-chip\"").expect("a chip");
    let to = page[from..].find("</span>").expect("its end");
    page[from..from + to].to_owned()
}

fn page(chip: Element) -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window, {chip} }
    }
}

#[expect(non_snake_case, reason = "a Dioxus component is a function named in CamelCase")]
fn Static() -> Element {
    page(rsx! { Chip { variant: ChipVariant::Accent, text: "Work" } })
}
#[expect(non_snake_case, reason = "a Dioxus component is a function named in CamelCase")]
fn Off() -> Element {
    page(rsx! {
        Chip { variant: ChipVariant::Neutral, text: "Work", pressed: Check::Off, onpress: |_| {} }
    })
}
#[expect(non_snake_case, reason = "a Dioxus component is a function named in CamelCase")]
fn On() -> Element {
    page(rsx! {
        Chip { variant: ChipVariant::Neutral, text: "Work", pressed: Check::On, onpress: |_| {} }
    })
}
#[expect(non_snake_case, reason = "a Dioxus component is a function named in CamelCase")]
fn Action() -> Element {
    page(rsx! { Chip { variant: ChipVariant::Neutral, text: "Work", onpress: |_| {} } })
}
#[expect(non_snake_case, reason = "a Dioxus component is a function named in CamelCase")]
fn Dotted() -> Element {
    page(rsx! {
        Chip {
            variant: ChipVariant::Neutral,
            text: "Work",
            dot: ChipDot::Colour("#336699".to_owned()),
        }
    })
}
#[expect(non_snake_case, reason = "a Dioxus component is a function named in CamelCase")]
fn TagDot() -> Element {
    page(rsx! {
        Chip { variant: ChipVariant::Neutral, text: "", dot: ChipDot::Tag(SystemHue::Red) }
    })
}
#[expect(non_snake_case, reason = "a Dioxus component is a function named in CamelCase")]
fn NamedDot() -> Element {
    page(rsx! {
        Chip {
            variant: ChipVariant::Neutral,
            text: "",
            dot: ChipDot::Tag(SystemHue::Gray),
            label: "Home".to_owned(),
            onpress: |_| {},
        }
    })
}
#[expect(non_snake_case, reason = "a Dioxus component is a function named in CamelCase")]
fn Removable() -> Element {
    page(
        rsx! { Chip { variant: ChipVariant::Label(LabelHue::Blue), text: "Work", onremove: |()| {} } },
    )
}

#[test]
fn a_chip_without_onpress_is_a_static_label() {
    let html = render(Static);
    assert!(!html.contains("<button"), "{html}");
    assert!(!html.contains("data-pressable"), "{html}");
    assert!(!html.contains("aria-pressed"), "{html}");
}

#[test]
fn a_pressable_chip_is_a_button_whose_aria_pressed_follows_pressed() {
    let off = render(Off);
    assert!(
        off.contains("<button type=\"button\" class=\"ds-chip-press\" aria-pressed=\"false\""),
        "{off}"
    );
    assert!(off.contains("data-state=\"off\""), "{off}");
    let on = render(On);
    assert!(on.contains("aria-pressed=\"true\""), "{on}");
    assert!(on.contains("data-state=\"on\""), "{on}");
}

#[test]
fn a_pressable_chip_with_no_state_is_an_action_with_no_aria_pressed() {
    let html = render(Action);
    assert!(html.contains("<button"), "{html}");
    assert!(!html.contains("aria-pressed"), "{html}");
}

#[test]
fn a_dot_is_a_disc_before_the_text_coloured_by_its_property() {
    let html = render(Dotted);
    assert!(
        html.contains("<i class=\"ds-chip-dot\" style=\"--chip-dot:#336699;\">"),
        "{html}"
    );
    assert!(html.find("ds-chip-dot") < html.find("Work"), "{html}");
}

#[test]
fn a_tag_dot_is_painted_from_its_token_and_a_dot_only_chip_is_named() {
    let html = render(TagDot);
    assert!(html.contains("--chip-dot:var(--tag-red);"), "{html}");
    assert!(html.contains("role=\"img\""), "{html}");
    assert!(html.contains("aria-label=\"Red\""), "{html}");
    let named = render(NamedDot);
    assert!(named.contains("--chip-dot:var(--tag-gray);"), "{named}");
    assert!(named.contains("aria-label=\"Home\""), "{named}");
    assert!(!named.contains("role=\"img\""), "{named}");
}

#[test]
fn a_chip_that_is_not_a_person_has_a_remove_button_after_its_text() {
    let html = render(Removable);
    assert!(html.contains("aria-label=\"Remove Work\""), "{html}");
    assert!(html.find("Work<") < html.find("ds-chip-remove"), "{html}");
}
