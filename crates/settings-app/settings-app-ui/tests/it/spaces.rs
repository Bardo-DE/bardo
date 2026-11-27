//! The Spaces page's list over a registry that answers from memory: Remove asks on the sheet
//! first, the choice reaches the registry as the person made it, and Cancel sends nothing. The
//! real link's order (memory is told before the registry is asked, and a delete choice with no
//! memory running goes no further) is tested over a private bus in settings-app-system.

use crate::support::Sandbox;
use crate::support::drive::click;
use settings_app_model::{PageSlug, SpaceKey, SpaceRow};
use settings_app_system::spaces::FakeSpaces;
use settings_app_ui::Opening;
use std::sync::Arc;
use std::time::Duration;
use ui_harness::Harness;

const TRIP: &str = "[aria-label=\"Remove Trip\"]";
const SHEET: &str = ".space-sheet";
const REMOVE: &str = ".space-sheet .ds-button[data-answers=\"return\"]";

fn spaces() -> Opening {
    Opening {
        page: Some(PageSlug("spaces".to_owned())),
        query: String::new(),
    }
}

/// Lets the page hear what the registry said: the follow loop runs on the harness's clock.
fn settle(harness: &mut Harness, what: &str, done: impl Fn(&Harness) -> bool) {
    for _ in 0..200 {
        if done(harness) {
            return;
        }
        harness.advance(Duration::from_millis(50));
    }
    panic!("{what} never happened:\n{}", harness.html());
}

struct World {
    fake: FakeSpaces,
    harness: Harness,
}

fn row(id: &str, name: &str) -> SpaceRow {
    SpaceRow {
        id: SpaceKey(id.to_owned()),
        name: name.to_owned(),
    }
}

fn world(name: &str) -> World {
    let fake = FakeSpaces::holding(vec![row("space-1", "Trip"), row("space-2", "Work")]);
    let sandbox = Sandbox::new(name).with_spaces(Arc::new(fake.clone()));
    let mut harness = sandbox.harness(spaces());
    settle(&mut harness, "the list", |h| h.count(TRIP) == 1);
    World { fake, harness }
}

/// What the registry was asked to remove, as `space <choice>` in order.
fn asked(w: &World) -> Vec<String> {
    w.fake
        .removals()
        .into_iter()
        .map(|(space, removal)| format!("{} {removal:?}", space.0))
        .collect()
}

fn open_sheet(w: &mut World) {
    click(&mut w.harness, TRIP);
    settle(&mut w.harness, "the sheet", |h| h.count(SHEET) == 1);
}

#[test]
fn the_sheet_opens_on_the_keeping_choices_and_says_its_words() {
    let mut w = world("spaces-sheet");
    assert_eq!(asked(&w), [] as [std::string::String; 0]);
    open_sheet(&mut w);
    let html = w.harness.html();
    for text in [
        "Remove \u{201c}Trip\u{201d}?",
        "Its memories move to the apps that saved them.",
        "Keep its history",
        "Delete its history",
        "History is what happened in this Space, like files you saved and messages you sent.",
        "Delete its memories instead",
    ] {
        assert!(html.contains(text), "{text}");
    }
    assert_eq!(
        w.harness
            .attr(".space-sheet [role=\"radio\"]", "aria-checked")
            .as_deref(),
        Some("true"),
        "the first choice is selected"
    );
    assert_eq!(
        w.harness.attr(REMOVE, "data-role").as_deref(),
        Some("normal"),
        "nothing is lost, so Remove is not destructive"
    );
}

#[test]
fn remove_asks_for_the_keeping_choices_and_the_row_goes() {
    let mut w = world("spaces-remove");
    open_sheet(&mut w);
    click(&mut w.harness, REMOVE);
    settle(&mut w.harness, "the row to go", |h| h.count(TRIP) == 0);
    assert_eq!(
        asked(&w),
        ["space-1 Removal { memories: MoveToApps, history: Keep }".to_owned()]
    );
    assert_eq!(w.harness.count("[aria-label=\"Remove Work\"]"), 1);
}
