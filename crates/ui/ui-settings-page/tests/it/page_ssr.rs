//! A small schema page as markup: each control a key kind gets, the app's own rows beside the
//! schema's and a page of nothing else. The markup lints clean and uses only styled `ds-` classes.

use crate::hygiene;
use std::collections::BTreeMap;

use dioxus::core::NoOpMutations;
use dioxus::prelude::*;
use settings_types::schema::{
    AgentSetting, AppId, Exposure, FilePath, ForeignTables, Help, KeyKind, KeyPath, KeySpec,
    Label as KeyLabel, Page, Schema, Section, WordLabels,
};
use ui_components::assembly::ds::Inject;
use ui_components::components::fields::field_row::FieldRow;
use ui_components::prelude::*;
use ui_lint::LintConfig;
use ui_settings_page::{SectionRows, SettingsPage};

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|word| (*word).to_owned()).collect()
}

fn key(path: &str, label: &str, section: &str, kind: KeyKind, default: &str) -> KeySpec {
    KeySpec {
        ends: None,
        path: KeyPath(path.to_owned()),
        kind,
        default: toml::Value::String(default.to_owned()),
        label: KeyLabel(label.to_owned()),
        help: Help(String::new()),
        page: Page::Appearance,
        section: Section(section.to_owned()),
        exposure: Exposure::Basic,
        labels: WordLabels::default(),
        agent: AgentSetting::HandsOff,
        unavailable: BTreeMap::default(),
        groups: BTreeMap::default(),
        places: BTreeMap::default(),
        note: None,
        control: None,
    }
}

fn schema() -> Schema {
    Schema {
        footnotes: Vec::new(),
        app: AppId("sample".to_owned()),
        file: FilePath("sample/sample.toml".to_owned()),
        version: 1,
        foreign: ForeignTables::default(),
        pages: Vec::default(),
        key: vec![
            key(
                "view.spelling",
                "Check spelling",
                "Writing",
                KeyKind::Toggle,
                "on",
            ),
            key(
                "view.marks",
                "Sender marks",
                "Writing",
                KeyKind::Segmented {
                    variants: words(&["icons", "letters"]),
                },
                "icons",
            ),
            key(
                "view.density",
                "Row density",
                "Reading",
                KeyKind::Segmented {
                    variants: words(&["compact", "comfortable", "roomy"]),
                },
                "compact",
            ),
            key(
                "view.signature",
                "Signature",
                "Reading",
                KeyKind::Text,
                "Sent from Sample",
            ),
        ],
    }
}

#[derive(Clone, PartialEq)]
struct StageProps {
    page: usize,
}

#[expect(
    non_snake_case,
    reason = "a Dioxus component function is named in CamelCase"
)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus hands a component its props by value"
)]
fn Stage(props: StageProps) -> Element {
    let body = (PAGES[props.page].1)();
    rsx! {
        Ds {
            appearance: Appearance { theme: Theme::Light, ..Appearance::default() },
            material: Material::Sheet,
            extent: RootExtent::Viewport,
            stylesheet: Inject::Host,
            {body}
        }
    }
}

fn own_row(label: &str) -> Element {
    rsx! { FieldRow { label: TextLine::from(label.to_owned()), Label { text: "Control+Option+K" } } }
}

type Page_ = (&'static str, fn() -> Element);

const PAGES: &[Page_] = &[
    ("schema", || {
        rsx! { SettingsPage { schemas: vec![schema()], onedit: |_| {} } }
    }),
    ("beside", || {
        let extras = vec![
            SectionRows::new("Writing", own_row("Next draft")),
            SectionRows::new("Shortcuts", own_row("Open settings")).footer("Per window."),
        ];
        rsx! { SettingsPage { schemas: vec![schema()], extras, onedit: |_| {} } }
    }),
    ("own-rows-only", || {
        let extras = vec![SectionRows::new("Keyboard", own_row("Open settings"))];
        rsx! { SettingsPage { extras } }
    }),
];

fn render(page: usize) -> String {
    let mut dom = VirtualDom::new_with_props(Stage, StageProps { page });
    dom.rebuild_in_place();
    for _ in 0..3 {
        dom.render_immediate(&mut NoOpMutations);
    }
    dioxus_ssr::render(&dom)
}

fn page(name: &str) -> String {
    let at = PAGES.iter().position(|(known, _)| *known == name);
    render(at.unwrap_or_else(|| panic!("no page {name}")))
}

#[test]
fn each_key_kind_gets_its_control() {
    let html = page("schema");
    assert!(
        html.contains("role=\"switch\""),
        "a switch for on/off: {html}"
    );
    assert!(
        html.contains("ds-segmented"),
        "segments for a choice of two: {html}"
    );
    assert!(html.contains("ds-popup"), "a pop-up for three: {html}");
    assert!(
        html.contains("Sent from Sample"),
        "a text key read-only: {html}"
    );
    // The two-word choice is segments, not a second switch.
    assert_eq!(html.matches("role=\"switch\"").count(), 1, "{html}");
}

#[test]
fn a_section_per_title_with_its_rows() {
    let html = page("schema");
    for title in ["Writing", "Reading"] {
        assert!(html.contains(title), "{title}: {html}");
    }
    assert_eq!(html.matches("ds-form-section-title").count(), 2, "{html}");
    for label in ["Check spelling", "Sender marks", "Row density", "Signature"] {
        assert!(html.contains(label), "{label}: {html}");
    }
}

#[test]
fn an_app_row_joins_its_section_and_an_unknown_section_stands_alone() {
    let html = page("beside");
    let next = html.find("Next draft").unwrap_or(0);
    let reading = html.find("Reading").unwrap_or(usize::MAX);
    assert!(
        next > 0 && next < reading,
        "joins Writing, before Reading: {html}"
    );
    assert!(
        html.contains("Shortcuts") && html.contains("Open settings"),
        "{html}"
    );
    assert!(html.contains("Per window."), "{html}");
    assert_eq!(html.matches("ds-form-section-title").count(), 3, "{html}");
}

#[test]
fn a_page_of_only_app_rows_keeps_the_form_look() {
    let html = page("own-rows-only");
    assert!(
        html.contains("ds-form") && html.contains("ds-form-section"),
        "{html}"
    );
    assert!(html.contains("Open settings"), "{html}");
    assert!(!html.contains("role=\"switch\""), "{html}");
}

#[test]
fn every_page_lints_clean_and_every_class_is_styled() {
    let sheet = ui_components::stylesheet();
    let config = LintConfig::new(&ui_components::kits());
    let failures: Vec<String> = PAGES
        .iter()
        .enumerate()
        .flat_map(|(at, (name, _))| hygiene::failures(name, &render(at), sheet, &config))
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
