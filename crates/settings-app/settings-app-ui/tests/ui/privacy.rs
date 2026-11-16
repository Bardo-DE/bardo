//! The Privacy & Security page over a fake compositor module in memory: the programs it lists are
//! named from desktop entries, each power shows where it stands, and a choice is sent as the one
//! change the compositor takes.

use crate::support::Sandbox;
use crate::support::compositor::{Compositor, Row};
use crate::support::drive::{click, scroll_into_view, sidebar_page};
use settings_app_model::PageSlug;
use settings_app_system::bus::Daemon;
use settings_app_system::modules::FakeModules;
use settings_app_ui::Opening;
use std::time::Duration;
use ui_harness::{Harness, Input};

const GRIM: &str = "/usr/bin/grim";
const OTHER: &str = "/opt/tools/capture";
const LOUPE: &str = "org.gnome.Loupe";

/// A power's row of a program: `privacy-<program>-<power>`, as the page names it.
fn line(program: &str, power: &str) -> String {
    format!("#privacy-{program}-{power}")
}

fn settle(harness: &mut Harness, what: &str, done: impl Fn(&Harness) -> bool) {
    // The fake daemon answers on real threads; a loaded machine running every test at once can
    // take several seconds, so the budget is generous (15 s) and costs nothing when it settles.
    for _ in 0..750 {
        if done(harness) {
            return;
        }
        harness.advance(Duration::from_millis(50));
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("{what} never happened:\n{}", harness.html());
}

struct World {
    fake: Compositor,
    harness: Harness,
}

/// The page, with the compositor listing `rows` in memory; none when `rows` is `None`.
fn world(name: &str, rows: Option<Vec<Row>>) -> World {
    let fake = Compositor::holding(rows.clone().unwrap_or_default());
    let modules = FakeModules::new();
    if rows.is_some() {
        modules.host(&Daemon::compositor(), fake.clone());
    }
    let sandbox = Sandbox::new(name).with_modules(modules);
    sandbox.write_application(
        "grim",
        "[Desktop Entry]\nType=Application\nName=Grim\nExec=/usr/bin/grim %f\n",
    );
    sandbox.write_application(
        "loupe",
        "[Desktop Entry]\nType=Application\nName=Image Viewer\nExec=flatpak run org.gnome.Loupe\nX-Flatpak=org.gnome.Loupe\n",
    );
    let harness = sandbox.harness(Opening {
        page: Some(PageSlug("privacy".to_owned())),
        query: String::new(),
    });
    World { fake, harness }
}

fn listed(world: &mut World) {
    settle(&mut world.harness, "the list", |h| {
        h.count("[data-privacy=\"list\"]") == 1
    });
}

fn label_of(harness: &Harness, row: &str) -> Option<String> {
    harness
        .text_of(&format!("{row} .ds-field-row-control .ds-button-label"))
        .or_else(|| harness.text_of(&format!("{row} .ds-field-row-control .ds-label")))
        .map(|text| text.trim().to_owned())
}

/// Opens the row's pop-up and clicks the item `nth` (from 1), the way a person points at it first.
fn choose(harness: &mut Harness, row: &str, nth: usize) {
    let button = format!("{row} .ds-popup .ds-button");
    scroll_into_view(harness, &button);
    click(harness, &button);
    let item = format!(".ds-menu-item:nth-child({nth}) .ds-menu-label");
    let at = harness.centre(&item).expect("the menu is open");
    harness.send(Input::pointer_move(at));
    click(harness, &item);
}

fn answers() -> Vec<Row> {
    vec![
        Row::person("screen", "aaa", GRIM, "on"),
        Row::person("clipboard", "bbb", GRIM, "off"),
        Row::person("windows", "ccc", LOUPE, "on"),
        Row::administrator("keys", "ddd", OTHER, "off"),
    ]
}

#[test]
fn the_page_is_in_the_sidebar_right_after_the_assistant_model() {
    let mut w = world("privacy-sidebar", Some(answers()));
    assert_eq!(
        w.harness
            .text_of(&sidebar_page("assistant-model"))
            .as_deref(),
        Some("Assistant Model")
    );
    assert_eq!(
        w.harness.text_of(&sidebar_page("privacy")).as_deref(),
        Some("Privacy & Security")
    );
    listed(&mut w);
}

#[test]
fn programs_are_named_from_their_entries_and_each_power_shows_where_it_stands() {
    let mut w = world("privacy-rows", Some(answers()));
    listed(&mut w);
    assert_eq!(w.harness.count("[data-program]"), 3);
    let names = w.harness.text_of(".privacy").unwrap_or_default();
    for name in ["Grim", "Image Viewer", "capture"] {
        assert!(names.contains(name), "{name} in {names}");
    }
    assert!(
        names.contains("In /opt/tools"),
        "a program no system entry names shows its folder: {names}"
    );
    let grim = "usr-bin-grim";
    // `(row, what it says)`.
    let cases = [
        (line(grim, "screen"), "Allowed"),
        (line(grim, "clipboard"), "Denied"),
        (line(grim, "keys"), "Ask"),
        (line("org-gnome-loupe", "windows"), "Allowed"),
        (line("opt-tools-capture", "keys"), "Denied"),
    ];
    for (row, want) in cases {
        assert_eq!(label_of(&w.harness, &row).as_deref(), Some(want), "{row}");
    }
    let page = w.harness.text_of(".privacy").unwrap_or_default();
    for power in [
        "Screen recording",
        "Keyboard input",
        "Typing help",
        "Clipboard",
        "Screen lock",
        "Displays",
        "Windows",
    ] {
        assert!(page.contains(power), "{power}");
    }
}

#[test]
fn a_choice_is_sent_as_the_one_change_and_the_list_follows() {
    let mut w = world("privacy-choose", Some(answers()));
    listed(&mut w);
    let screen = line("usr-bin-grim", "screen");
    // The menu: Allowed, Denied, Ask.
    choose(&mut w.harness, &screen, 2);
    settle(&mut w.harness, "the answer", |h| {
        label_of(h, &screen).as_deref() == Some("Denied")
    });
    assert_eq!(w.fake.log(), ["set privacy.screen.aaa off"]);

    let clipboard = line("usr-bin-grim", "clipboard");
    choose(&mut w.harness, &clipboard, 3);
    settle(&mut w.harness, "the question to come back", |h| {
        label_of(h, &clipboard).as_deref() == Some("Ask")
    });
    assert_eq!(
        w.fake.log(),
        [
            "set privacy.screen.aaa off",
            "forget privacy.clipboard.bbb.forget"
        ]
    );
}
