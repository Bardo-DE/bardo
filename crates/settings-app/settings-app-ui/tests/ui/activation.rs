//! Showing a page asks the bus to start the daemons that page reads, and only those: a bus with a
//! service file for each daemon records which were started, and a name with no service file stays
//! absent, so the page says it is unavailable as before.

use crate::support::Sandbox;
use crate::support::bus::PrivateBus;
use crate::support::drive::{let_threads_run, settle_on_threads};
use account_wire::names::{ACCOUNTS_BUS, INFERENCE_BUS};
use memory_dbus::MEMORY_BUS;
use settings_app_model::PageSlug;
use settings_app_ui::Opening;
use sync_bus::SYNC_BUS;
use ui_harness::Harness;

/// `MEMORY_BUS` is a daemon no page reads here: starting it would be a start for a page nobody opened.
const ALL: [&str; 4] = [ACCOUNTS_BUS, INFERENCE_BUS, SYNC_BUS, MEMORY_BUS];

fn page(slug: &str) -> Opening {
    Opening {
        page: Some(PageSlug(slug.to_owned())),
        query: String::new(),
    }
}

/// Shows `slug` on a bus that can start `services`, lets the window run until the bus has started
/// the `expected` daemons and returns the names it started, with the window. A page that starts
/// nothing has nothing to wait for: the window then runs a fair while (best effort).
fn visit(test: &str, slug: &str, services: &[&str], expected: &[&str]) -> (Vec<String>, Harness) {
    let bus = PrivateBus::start_with_services(test, services);
    let mut harness = Sandbox::new(test).on_bus(&bus.address).harness(page(slug));
    // The starts are asked of the bus on real threads.
    if expected.is_empty() {
        let_threads_run(&mut harness, 100);
    } else {
        settle_on_threads(&mut harness, "the daemons were started", |_| {
            bus.started() == expected
        });
        // Anything started beyond them goes in the same round; give it a moment to show.
        let_threads_run(&mut harness, 10);
    }
    (bus.started(), harness)
}

#[test]
fn the_accounts_page_starts_the_three_daemons_it_reads() {
    let (started, _) = visit(
        "activate-accounts",
        "accounts",
        &ALL,
        &[ACCOUNTS_BUS, INFERENCE_BUS, SYNC_BUS],
    );
    assert_eq!(started, [ACCOUNTS_BUS, INFERENCE_BUS, SYNC_BUS]);
}

#[test]
fn the_intelligence_page_starts_the_daemons_it_reads_and_not_memory() {
    let (started, _) = visit(
        "activate-intelligence",
        "intelligence",
        &ALL,
        &[ACCOUNTS_BUS, INFERENCE_BUS, SYNC_BUS],
    );
    assert_eq!(started, [ACCOUNTS_BUS, INFERENCE_BUS, SYNC_BUS]);
}

#[test]
fn the_spaces_page_starts_only_the_registry() {
    let (started, _) = visit("activate-spaces", "spaces", &ALL, &[ACCOUNTS_BUS]);
    assert_eq!(started, [ACCOUNTS_BUS]);
}

#[test]
fn a_page_that_reads_files_only_starts_nothing() {
    let (started, _) = visit("activate-appearance", "appearance", &ALL, &[]);
    assert_eq!(started, Vec::<String>::new());
}

#[test]
fn a_name_without_a_service_file_stays_absent_and_the_page_says_so() {
    let (started, mut harness) = visit(
        "activate-unknown",
        "accounts",
        &[INFERENCE_BUS],
        &[INFERENCE_BUS],
    );
    assert_eq!(
        started,
        [INFERENCE_BUS],
        "only the name that can be started"
    );
    settle_on_threads(&mut harness, "the pane says unavailable", |h| {
        h.attr(".accounts", "data-accounts").as_deref() == Some("unavailable")
    });
    assert_eq!(
        harness.attr(".accounts", "data-accounts").as_deref(),
        Some("unavailable"),
        "{}",
        harness.html()
    );
    assert!(
        harness
            .html()
            .contains("The accounts service is not running"),
        "no daemon took its name, so the pane says none is running"
    );
}
