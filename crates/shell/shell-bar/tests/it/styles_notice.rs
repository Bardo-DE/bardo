//! The bar's notice that the person's styles were turned off: nothing while the styles load, the
//! one sentence and its Turn On Again button when the crash guard left them off, no sheet drawn
//! then, and a press writes the feed's `turn_on` for the daemon. Guards live in a scratch folder.

use std::time::Duration;

use base_testkit::Scratch;
use shell_bar::Bar;
use shell_ui_kit::harness::SurfaceHarness;
use shell_ui_kit::links::ControlCenterLink;
use ui_components::style::kit::{Styles, UserStyle};
use ui_harness::Input;
use ui_host_hooks::StylesGuard;

const CSS: &str = ".ds { --accent: red }";
const SENTENCE: &str = "Your styles were turned off because bardo-shell could not start with them.";
const BUTTON: &str = ".ds-inline-banner .ds-button";

fn styles() -> Styles {
    Styles {
        desktop: UserStyle(CSS.to_owned()),
        ..Styles::default()
    }
}

/// The guard of the `starts`th start with the same sheets, none of the earlier ones good.
fn guard_after(dir: &Scratch, starts: usize) -> StylesGuard {
    let styles = styles();
    // Every start is recorded in turn, so each one must run: the last guard is the answer.
    (0..starts)
        .fold(None, |_, _| {
            Some(StylesGuard::begin(dir.path(), "shell", &styles))
        })
        .unwrap_or_else(|| panic!("no starts"))
}

fn start(guard: StylesGuard) -> SurfaceHarness<Bar> {
    let mut bar = SurfaceHarness::<Bar>::setup(ControlCenterLink::new(), ())
        .desktop_style(CSS)
        .guard(guard)
        .start();
    bar.harness.advance(Duration::from_millis(60));
    bar
}

#[test]
fn a_start_with_its_styles_shows_no_notice_and_draws_them() {
    let dir = Scratch::new("styles-notice-with");
    let guard = guard_after(&dir, 1);
    assert!(guard.load());
    let html = start(guard).harness.html();
    assert!(!html.contains(SENTENCE), "{html}");
    assert!(html.contains("data-ds-style=\"desktop\""), "{html}");
}

#[test]
fn a_start_the_guard_turned_the_styles_off_for_says_so_and_draws_none() {
    let dir = Scratch::new("styles-notice-without");
    let guard = guard_after(&dir, 3);
    assert!(!guard.load());
    let html = start(guard).harness.html();
    assert!(html.contains(SENTENCE), "{html}");
    assert!(html.contains("Turn On Again"), "{html}");
    assert!(!html.contains("data-ds-style"), "{html}");
}

#[test]
fn turn_on_again_asks_the_daemon() {
    let dir = Scratch::new("styles-notice-press");
    let mut bar = start(guard_after(&dir, 3));
    assert_eq!(bar.turn_on.latest(), None);
    let at = bar
        .harness
        .centre(BUTTON)
        .unwrap_or_else(|| panic!("no button: {}", bar.harness.html()));
    bar.harness.send(Input::click(at));
    bar.harness.advance(Duration::from_millis(40));
    assert_eq!(bar.turn_on.latest().map(|sent| sent.seq), Some(1));
}
