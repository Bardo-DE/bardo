//! Choosing an engine that is not there in the Local row: the sheet asks to download it, "Not Now"
//! leaves everything as it was, "Download" starts one and follows it, a Strata the person has is
//! pointed at with the chooser, and an engine that is there asks nothing. Over a fake inferd
//! module in memory and a desk that writes down what the sheet asks of the computer.

use crate::support::Sandbox;
use crate::support::desk::{Fake, nothing_installed};
use crate::support::drive::settle_on_threads;
use crate::support::drive::{click, scroll_into_view};
use crate::support::inferd::Inferd;
use settings_app_model::{ASSISTANT_MODEL_SLUG, EngineStanding, InstallScope, PageSlug};
use settings_app_ui::Opening;
use ui_harness::{Harness, Input};
use ui_settings::live::Verdict;

const MODEL_BUTTON: &str = ".assistant-model .ds-popup .ds-button";
const ENGINE_BUTTON: &str = "#picker-engine-ai-model-text-balanced .ds-popup .ds-button";
const SHEET: &str = "[data-engine-sheet]";
const GO: &str = ".engine-sheet-buttons .ds-button[data-answers=\"return\"]";
const LEAVE: &str = ".engine-sheet-buttons .ds-button[data-answers=\"escape\"]";

/// Clicks the menu item `nth` (from 1), the way a person points at it first.
fn pick(harness: &mut Harness, nth: usize) {
    let label = format!(".ds-menu-item:nth-child({nth}) .ds-menu-label");
    let at = harness.centre(&label).expect("the menu is open");
    harness.send(Input::pointer_move(at));
    click(harness, &label);
}

struct World {
    desk: Fake,
    harness: Harness,
}

/// The assistant's page with Local chosen, over a computer whose engines `desk` lists.
fn world(name: &str, desk: Fake) -> World {
    let fake = Inferd::new(Verdict::Allow).on_slots();
    let harness = Sandbox::new(name)
        .with_modules(fake.hosted())
        .with_engines(desk.desk())
        .harness(Opening {
            page: Some(PageSlug(ASSISTANT_MODEL_SLUG.to_owned())),
            query: String::new(),
        });
    let mut world = World { desk, harness };
    settle_on_threads(&mut world.harness, "the picker", |h| {
        h.attr(".picker", "data-picker").as_deref() == Some("ready")
    });
    scroll_into_view(&mut world.harness, MODEL_BUTTON);
    click(&mut world.harness, MODEL_BUTTON);
    // Items: None, Automatic, Local.
    pick(&mut world.harness, 3);
    settle_on_threads(&mut world.harness, "the engines", |h| {
        h.count(ENGINE_BUTTON) == 1
    });
    world
}

/// Opens the engine menu and picks the item `nth`.
fn choose_engine(world: &mut World, nth: usize) {
    click(&mut world.harness, ENGINE_BUTTON);
    pick(&mut world.harness, nth);
}

#[test]
fn an_engine_that_is_not_there_asks_to_be_downloaded_and_not_now_leaves_everything_as_it_was() {
    let mut world = world("engine-ask", Fake::with(nothing_installed()));
    // Items: llama.cpp (it runs the models listed), then vLLM and Strata, which could be got.
    choose_engine(&mut world, 2);
    settle_on_threads(&mut world.harness, "the sheet", |h| h.count(SHEET) == 1);
    let html = world.harness.html();
    assert!(html.contains("Download vLLM?"), "{html}");
    assert!(
        html.contains(
            "Your assistant needs vLLM to run models on this computer. It's about 30 MB."
        ),
        "{html}"
    );
    assert!(
        html.contains("All Users"),
        "everyone's is the default: {html}"
    );
    assert!(html.contains("Install for"), "{html}");
    click(&mut world.harness, LEAVE);
    assert_eq!(world.harness.count(SHEET), 0, "the sheet is gone");
    assert!(world.desk.installs().is_empty(), "nothing was started");
    assert!(world.desk.points().is_empty());
    assert_eq!(world.desk.restarts(), 0);
}

#[test]
fn download_starts_one_for_everyone_follows_it_and_continue_restarts_the_assistant() {
    let mut world = world("engine-download", Fake::with(nothing_installed()));
    choose_engine(&mut world, 2);
    settle_on_threads(&mut world.harness, "the sheet", |h| h.count(SHEET) == 1);
    click(&mut world.harness, GO);
    assert_eq!(
        world.desk.installs(),
        [("vllm".to_owned(), InstallScope::AllUsers)]
    );
    assert_eq!(
        world.harness.attr(SHEET, "data-engine-sheet").as_deref(),
        Some("working")
    );
    let mut rows = nothing_installed();
    rows[1].standing = EngineStanding::Installing(500);
    world.desk.show(rows.clone());
    settle_on_threads(&mut world.harness, "the bar", |h| {
        h.html().contains("Downloading vLLM")
    });
    rows[1].standing = EngineStanding::Ready;
    world.desk.show(rows);
    settle_on_threads(&mut world.harness, "the end", |h| {
        h.attr(SHEET, "data-engine-sheet").as_deref() == Some("done")
    });
    assert!(world.harness.html().contains("vLLM Is Ready"));
    click(&mut world.harness, GO);
    settle_on_threads(&mut world.harness, "the restart", |_| {
        world.desk.restarts() >= 1
    });
    assert_eq!(world.desk.restarts(), 1, "the assistant restarts once");
    assert_eq!(world.harness.count(SHEET), 0);
}
