//! The banner at the head of an account that needs the person: its sign-in ended, or the provider
//! is about to end it. Over a fake accountd module in memory.

use crate::support::Sandbox;
use crate::support::accountd::{Accountd, Held};
use crate::support::drive::click;
use settings_app_model::PageSlug;
use settings_app_ui::{Opening, STYLE};
use std::time::Duration;
use ui_harness::Harness;
use ui_lint::{LintConfig, markup};
use ui_settings::live::Verdict;

const BANNER: &str = "[data-accounts=\"attention\"]";
const BANNER_BUTTON: &str = "[data-accounts=\"attention\"] .ds-button";
const SIGN_IN_ROW: &str = "[aria-label=\"Sign in again\"] .ds-field-row-control .ds-button";
const EXPIRY_ROW: &str = "[aria-label=\"Sign-in\"]";
const ENDING: &str = "Google signs this account out on 2026-10-14";
const ENDED: &str = "Signed out by Google on 2026-10-14";

fn settle(harness: &mut Harness, what: &str, done: impl Fn(&Harness) -> bool) {
    // The fake daemon answers on a real thread; a loaded machine running every test at once can
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

/// A Google account with the given standing and expiry words.
fn google(id: &str, label: &str, state: &str, words: Option<&str>) -> Held {
    let held = Held::new(id, label, state, &["mail"]).of_provider("google", "Google");
    match words {
        Some(words) => held.expiring(words),
        None => held,
    }
}

/// The page over a fake accountd holding `rows`, with the account `id` chosen.
struct World {
    harness: Harness,
    fake: Accountd,
}

fn world(name: &str, rows: Vec<Held>, chosen: &str) -> World {
    let count = rows.len();
    let fake = Accountd::new(Verdict::Allow, rows);
    let opening = Opening {
        page: Some(PageSlug("accounts".to_owned())),
        query: String::new(),
    };
    let mut harness = Sandbox::new(name)
        .with_modules(fake.hosted())
        .harness_in(opening, ui_components::prelude::Scheme::Light);
    settle(&mut harness, "the list", |h| {
        h.count(".account-row-body") >= count
    });
    click(
        &mut harness,
        &format!("#account-{}", chosen.replace('.', "-")),
    );
    settle(&mut harness, "the account", |h| {
        h.attr(".accounts-detail", "data-account").as_deref() == Some(chosen)
    });
    World { harness, fake }
}

#[test]
fn a_sign_in_that_ended_or_will_end_says_so_in_a_banner_that_replaces_the_rows() {
    // (name, standing, the provider's words, the banner's sentence, the list row's dot, the
    // button's label when it names the account)
    let cases = [
        (
            "banner-ended",
            "needs_reauth",
            ENDED,
            "Sign in again to keep using Work mail.",
            "Needs sign-in",
            Some("Sign In\u{2026} Work mail"),
        ),
        (
            "banner-ending",
            "ok",
            ENDING,
            "Google will sign this account out on 14 Oct.",
            "Sign-in ends soon",
            None,
        ),
    ];
    for (name, state, words, sentence, dot, button) in cases {
        let rows = vec![google("work.example", "Work mail", state, Some(words))];
        let w = world(name, rows, "work.example");
        assert_eq!(w.harness.count(BANNER), 1, "{name}: {}", w.harness.html());
        assert!(
            w.harness.html().contains(sentence),
            "{name}: {}",
            w.harness.html()
        );
        assert_eq!(
            w.harness.count(SIGN_IN_ROW),
            0,
            "{name}: the row moved into the banner"
        );
        assert_eq!(
            w.harness.count(EXPIRY_ROW),
            0,
            "{name}: one message, not two"
        );
        if let Some(button) = button {
            assert_eq!(
                w.harness.attr(BANNER_BUTTON, "aria-label").as_deref(),
                Some(button),
                "{name}"
            );
        }
        assert_eq!(
            w.harness
                .count(&format!("#account-work-example [aria-label=\"{dot}\"]")),
            1,
            "{name}: the list row carries the dot"
        );
    }
}

#[test]
fn an_account_that_needs_nothing_keeps_its_row_and_has_no_banner() {
    let rows = vec![google("home", "Home cloud", "ok", None)];
    let w = world("banner-none", rows, "home");
    assert_eq!(w.harness.count(BANNER), 0);
    assert_eq!(w.harness.count(SIGN_IN_ROW), 1);
    assert_eq!(
        w.harness
            .count("#account-home [aria-label=\"Needs sign-in\"]"),
        0
    );
    assert_eq!(w.harness.count("#account-home .ds-badge"), 0);
}

#[test]
fn the_banners_button_starts_the_sign_in_in_accountd_for_either_banner() {
    for (name, state, words) in [
        ("banner-press-ended", "needs_reauth", ENDED),
        ("banner-press-ending", "ok", ENDING),
    ] {
        let rows = vec![google("work.example", "Work mail", state, Some(words))];
        let mut w = world(name, rows, "work.example");
        assert!(w.fake.sets().is_empty(), "{name}: nothing was set yet");
        click(&mut w.harness, BANNER_BUTTON);
        settle(&mut w.harness, "the action", |_| !w.fake.sets().is_empty());
        assert_eq!(
            w.fake.sets(),
            [(
                "accounts.work.example.reauth".to_owned(),
                toml::Value::Boolean(true)
            )],
            "{name}"
        );
    }
}

#[test]
fn the_banner_is_only_quire_components_and_layout_only_css() {
    let rows = vec![google(
        "work.example",
        "Work mail",
        "needs_reauth",
        Some(ENDED),
    )];
    let w = world("banner-lint", rows, "work.example");
    assert_eq!(w.harness.count(BANNER), 1);
    let css = format!("{}\n{STYLE}", ui_components::stylesheet());
    let offences = markup(
        &w.harness.html(),
        &css,
        &LintConfig::new(&ui_components::kits()),
    );
    assert!(offences.is_empty(), "{offences:#?}");
}
