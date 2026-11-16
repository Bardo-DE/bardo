//! The Software Update page over a service that answers from memory: what each standing shows,
//! what a press asks of the service, and the page without a service.

use crate::support::Sandbox;
use crate::support::drive::{click, page_of, title_of};
use settings_app_model::{
    ByteCount, Permille, ProblemKind, UnixSeconds, UpdateOffer, UpdatePhase, UpdateStatus,
    UpdateVersion,
};
use settings_app_system::updates::{FakeUpdates, NOW, UpdatesAsked};
use settings_app_ui::Opening;
use ui_harness::Harness;
use std::time::Duration;

const ROW: &str = ".updates .ds-field-row";
const BUTTONS: &str = ".updates .ds-field-row .ds-button";
const CHECK: &str = "[aria-label=\"Check for Updates\"]";
const UPDATE_NOW: &str = "[aria-label=\"Update Now\"]";
const LOG_OUT: &str = "[aria-label=\"Log Out\u{2026}\"]";
const BAR: &str = ".updates-meter .ds-progress";

fn software_update() -> Opening {
    Opening {
        page: Some(settings_app_model::PageSlug("software-update".to_owned())),
        query: String::new(),
    }
}

fn idle() -> UpdateStatus {
    UpdateStatus {
        phase: UpdatePhase::Idle,
        current: Some("0.3.1".to_owned()),
        offer: None,
        progress: Permille(0),
        last_checked: Some(UnixSeconds(NOW.0 - 300)),
        problem: None,
        problem_kind: ProblemKind::Other,
    }
}

fn version(text: &str) -> UpdateVersion {
    UpdateVersion::parse(text).expect("a version")
}

fn offered_version(phase: UpdatePhase, text: &str) -> UpdateStatus {
    UpdateStatus {
        phase,
        offer: Some(UpdateOffer {
            version: version(text),
            notes: "Faster start.\nFewer crashes.".to_owned(),
            size: ByteCount(45_600_000),
        }),
        ..idle()
    }
}

fn offered(phase: UpdatePhase) -> UpdateStatus {
    offered_version(phase, "0.4.0")
}

/// Lets the page hear what the service said: the follow loop runs on the harness's clock.
fn settle(harness: &mut Harness, what: &str, done: impl Fn(&Harness) -> bool) {
    for _ in 0..200 {
        if done(harness) {
            return;
        }
        harness.advance(Duration::from_millis(50));
    }
    panic!("{what} never happened:\n{}", harness.html());
}

fn showing(harness: &mut Harness, what: &str, selector: &'static str) {
    settle(harness, what, |h| h.count(selector) > 0);
}

fn row_text(harness: &Harness) -> String {
    harness.text_of(ROW).unwrap_or_default()
}

fn world(name: &str, service: &FakeUpdates) -> (Sandbox, Harness) {
    let sandbox = Sandbox::new(name).with_service(service.clone());
    let harness = sandbox.harness(software_update());
    (sandbox, harness)
}

#[test]
fn the_page_opens_from_its_slug_and_says_plainly_when_nothing_serves_updates() {
    let sandbox = Sandbox::new("swupdate-absent");
    let mut harness = sandbox.harness(software_update());
    showing(
        &mut harness,
        "the unavailable row",
        "[data-updates=\"unavailable\"]",
    );
    assert_eq!(page_of(&harness).as_deref(), Some("software-update"));
    assert_eq!(title_of(&harness).as_deref(), Some("Software Update"));
    let row = harness
        .text_of("[data-updates=\"unavailable\"]")
        .unwrap_or_default();
    assert!(
        row.contains("Software updates are not available on this computer."),
        "{row}"
    );
    assert_eq!(harness.count(BUTTONS), 0, "nothing to press");
}

#[test]
fn checking_asks_the_service_shows_a_spinner_and_then_what_it_found() {
    let service = FakeUpdates::answering(idle());
    let (_sandbox, mut harness) = world("swupdate-check", &service);
    showing(&mut harness, "the button", CHECK);
    assert_eq!(harness.attr(CHECK, "aria-busy"), None, "not busy before");

    click(&mut harness, CHECK);
    assert_eq!(service.asked(), [UpdatesAsked::Check]);
    settle(&mut harness, "the busy button", |h| {
        h.attr(CHECK, "aria-busy").as_deref() == Some("true")
    });
    assert!(row_text(&harness).contains("Checking for updates"));

    service.becomes(offered(UpdatePhase::Available));
    showing(&mut harness, "the offer", UPDATE_NOW);
    let row = row_text(&harness);
    assert!(row.contains("Quire 0.4.0"), "{row}");
    assert!(row.contains("45.6 MB"), "{row}");
    assert_eq!(
        harness.count(CHECK),
        0,
        "an offer replaces the check button"
    );
}

#[test]
fn update_now_downloads_with_a_bar_installs_without_one_and_ends_with_log_out() {
    let service = FakeUpdates::answering(offered(UpdatePhase::Available));
    let (sandbox, mut harness) = world("swupdate-install", &service);
    showing(&mut harness, "Update Now", UPDATE_NOW);

    click(&mut harness, UPDATE_NOW);
    assert_eq!(service.asked(), [UpdatesAsked::Install(version("0.4.0"))]);
    settle(&mut harness, "the download", |h| {
        row_text(h).contains("Downloading")
    });
    assert_eq!(
        harness.count(BUTTONS),
        0,
        "nothing to press while it downloads"
    );

    service.becomes(UpdateStatus {
        progress: Permille(250),
        ..offered(UpdatePhase::Downloading)
    });
    settle(&mut harness, "a quarter", |h| {
        h.attr(BAR, "aria-valuenow").as_deref() == Some("25")
    });
    assert_eq!(
        harness.text_of(".updates-meter").as_deref(),
        Some("11.4 MB of 45.6 MB")
    );

    service.becomes(offered(UpdatePhase::Installing));
    settle(&mut harness, "the pending bar", |h| {
        h.count(BAR) == 1 && h.attr(BAR, "aria-valuenow").is_none()
    });
    assert!(row_text(&harness).contains("Installing"));

    service.becomes(UpdateStatus {
        phase: UpdatePhase::Installed,
        current: Some("0.4.0".to_owned()),
        offer: None,
        ..idle()
    });
    showing(&mut harness, "Log Out", LOG_OUT);
    let row = row_text(&harness);
    assert!(row.contains("Quire 0.4.0"), "{row}");
    assert!(row.contains("Log out to finish the update"), "{row}");
    assert_eq!(sandbox.leaves().started(), 0, "nothing logs out by itself");

    click(&mut harness, LOG_OUT);
    assert_eq!(sandbox.leaves().started(), 1);
}
