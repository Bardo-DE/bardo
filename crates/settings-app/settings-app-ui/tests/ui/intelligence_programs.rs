//! The Intelligence page over the schemas docket, almanac and cua ship: each row in the section
//! its own `section` names, a write in the program's own file, and rows that do not fit left alone.

use crate::support::Sandbox;
use crate::support::drive::{click, scroll_into_view};
use settings_app_model::PageSlug;
use settings_app_ui::Opening;
use ui_harness::Harness;

fn intelligence() -> Opening {
    Opening {
        page: Some(PageSlug("intelligence".to_owned())),
        query: String::new(),
    }
}

/// The page's markup with entities for an apostrophe read back, cut where the Advanced
/// group starts: what is drawn before it, and what it holds.
fn basic_and_advanced(harness: &Harness) -> (String, String) {
    let html = harness.html().replace("&#39;", "'");
    let cut = html.find("class=\"advanced\"").expect("the advanced group");
    (html[..cut].to_owned(), html[cut..].to_owned())
}

/// Each of `texts` is on the page after the one before it.
fn in_order(html: &str, texts: &[&str]) {
    let mut from = 0;
    for text in texts {
        let at = html[from..]
            .find(&format!(">{text}<"))
            .unwrap_or_else(|| panic!("{text:?} is not drawn after byte {from}"));
        from += at;
    }
}

/// `(section, the rows it shows before Advanced)`, in the order the page draws them.
const BASIC: &[(&str, &[&str])] = &[
    (
        "Assistant",
        &["New conversations start in", "Other apps", "Coding agents"],
    ),
    (
        "Auto",
        &[
            "Changes it can undo, within what you asked",
            "Other changes it can undo",
            "Sending",
            "Deleting",
            "Using another app's window",
        ],
    ),
    (
        "Manual",
        &[
            "Changes it can undo, within what you asked",
            "Other changes it can undo",
            "Sending",
            "Deleting",
            "Using another app's window",
        ],
    ),
    (
        "Models",
        &[
            "Keep AI on this computer",
            "Count my other machines as this computer",
            "Let my other computers use this computer's models",
        ],
    ),
    ("Memory", &["Memory files", "Tidy memory"]),
    (
        "Computer use",
        &[
            "Let the assistant use my apps for me",
            "Apps it may use",
            "Where it works",
            "Pause when I touch the window",
        ],
    ),
];

#[test]
fn each_basic_row_is_drawn_in_the_section_its_schema_names() {
    let sandbox = Sandbox::new("programs-basic");
    let harness = sandbox.harness(intelligence());
    let (basic, _) = basic_and_advanced(&harness);
    for (section, rows) in BASIC {
        let mut texts = vec![*section];
        texts.extend(rows.iter().copied());
        in_order(&basic, &texts);
    }
    let sections: Vec<&str> = BASIC.iter().map(|(section, _)| *section).collect();
    in_order(&basic, &sections);
}

/// Clicks the switch of `label`, scrolled into view.
fn flip(harness: &mut Harness, label: &str) {
    let toggle = format!("[aria-label=\"{label}\"] .ds-toggle");
    scroll_into_view(harness, &toggle);
    click(harness, &toggle);
}

#[test]
fn a_write_lands_in_the_file_of_the_program_that_owns_the_row() {
    let sandbox = Sandbox::new("programs-writes");
    let mut harness = sandbox.harness(intelligence());
    for file in [
        "docket/settings.toml",
        "almanac/settings.toml",
        "cua/settings.toml",
    ] {
        assert_eq!(sandbox.read(file), None, "{file}: looking writes nothing");
    }

    // Top to bottom, because the page is only ever scrolled down: the Assistant section first.
    flip(&mut harness, "Other apps");
    let docket = sandbox.read("docket/settings.toml").expect("docket's file");
    assert!(docket.contains("expose = true"), "{docket}");
    assert!(
        !docket.contains("autopause") && !docket.contains("when"),
        "{docket}"
    );
    assert_eq!(sandbox.read("almanac/settings.toml"), None);
    assert_eq!(sandbox.read("cua/settings.toml"), None);

    let tidy = ".ds-segmented[aria-label=\"Tidy memory\"] .ds-segmented-segment:nth-of-type(2)";
    scroll_into_view(&mut harness, tidy);
    click(&mut harness, tidy);
    let almanac = sandbox
        .read("almanac/settings.toml")
        .expect("almanac's file");
    assert!(almanac.contains("when = \"manual\""), "{almanac}");
    assert_eq!(sandbox.read("cua/settings.toml"), None);

    flip(&mut harness, "Pause when I touch the window");
    let cua = sandbox.read("cua/settings.toml").expect("cua's file");
    assert!(cua.contains("autopause = false"), "{cua}");
    assert!(!cua.contains("when"), "{cua}");

    assert_eq!(
        sandbox.read("inferd/inferd.toml"),
        None,
        "inferd's file is not touched"
    );
}

#[test]
fn lending_this_computers_models_is_off_until_the_switch_is_flipped() {
    const LABEL: &str = "Let my other computers use this computer's models";
    let sandbox = Sandbox::new("programs-tailnet-serve");
    let mut harness = sandbox.harness(intelligence());
    let toggle = format!("[aria-label=\"{LABEL}\"] .ds-toggle");
    scroll_into_view(&mut harness, &toggle);
    assert_eq!(
        harness.attr(&toggle, "aria-checked").as_deref(),
        Some("false"),
        "off by default"
    );
    assert_eq!(
        sandbox.read("inferd/inferd.toml"),
        None,
        "looking writes nothing"
    );
    // Turning it on is asked about first (tests/ui/lending.rs drives the answer).
    click(&mut harness, &toggle);
    assert_eq!(
        sandbox.read("inferd/inferd.toml"),
        None,
        "the press alone writes nothing"
    );
}

#[test]
fn rows_that_do_not_fit_are_shown_as_written_and_never_rewritten() {
    let sandbox = Sandbox::new("programs-rows-foreign");
    let written = "[cua]\napps = [{ app = \"org.a.B\", allow = \"ask\", note = \"mine\" }]\n";
    sandbox.write("cua/settings.toml", written);
    let harness = sandbox.harness(intelligence());
    assert_eq!(harness.count(".key-rows"), 0);
    assert!(harness.html().contains("Edit in file"));
    assert_eq!(sandbox.read("cua/settings.toml").as_deref(), Some(written));
}
