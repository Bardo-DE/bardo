//! The Privacy page's screen-sharing apps over a fake portal on a private bus: the apps it
//! remembers are listed by the id it proved them by, Remove takes one back and the row goes, and a
//! refusal is told. The rest is tested in `settings-app-model`.

use crate::support::Sandbox;
use crate::support::bus::PrivateBus;
use crate::support::compositor::{Compositor, Row};
use crate::support::drive::click;
use crate::support::portal::{Fake, Remembered, serve};
use settings_app_client::Vouch;
use settings_app_model::PageSlug;
use settings_app_system::bus::Daemon;
use settings_app_system::modules::FakeModules;
use settings_app_system::updates::NOW;
use settings_app_ui::Opening;
use std::time::Duration;
use ui_harness::Harness;
use ui_settings::live::AnyOwner;

const MEET: &str = "org.example.Meet";
const CALL: &str = "org.example.Call";
const GRABBER: &str = "org.evil.Grabber";
const HOUR: u64 = 3600;
const DAY: u64 = 24 * HOUR;

const GROUP: &str = "[data-screen-sharing=\"list\"]";
const REMOVE_MEET: &str = "[aria-label=\"Remove Video Meet\"]";

fn privacy() -> Opening {
    Opening {
        page: Some(PageSlug("privacy".to_owned())),
        query: String::new(),
    }
}

fn settle(harness: &mut Harness, what: &str, done: impl Fn(&Harness) -> bool) {
    // The fake daemons answer on real threads; a loaded machine running every test at once can
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

/// The apps the portal remembers: one a system desktop entry names, one that has no entry, and
/// one whose entry the portal reads as "System Settings".
fn remembered() -> Vec<Remembered> {
    let ago = |seconds: u64| NOW.0.unsigned_abs() - seconds;
    vec![
        Remembered {
            app: MEET,
            label: "Meet",
            mode: 2,
            granted: ago(3 * DAY),
        },
        Remembered {
            app: CALL,
            label: "Call",
            mode: 1,
            granted: ago(2 * HOUR),
        },
        Remembered {
            app: GRABBER,
            label: "System Settings",
            mode: 2,
            granted: ago(DAY),
        },
    ]
}

struct World {
    portal: Option<Fake>,
    harness: Harness,
    _rt: tokio::runtime::Runtime,
    _bus: PrivateBus,
}

/// The Privacy page over a compositor listing one program, and a portal on the bus holding `apps`.
fn world(name: &str, apps: &[Remembered]) -> World {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("a runtime for the fakes");
    let bus = PrivateBus::start(name);
    // The compositor's module is in memory: the portal is the bus's, and no live module.
    let modules = FakeModules::new();
    modules.host(
        &Daemon::compositor(),
        Compositor::holding(vec![Row::person("screen", "aaa", "/usr/bin/grim", "on")]),
    );
    let portal = Some(rt.block_on(serve(&bus, apps)));
    let vouch = Vouch::of(AnyOwner);
    let sandbox = Sandbox::new(name)
        .on_bus_vouched(&bus.address, vouch)
        .with_modules(modules);
    sandbox.write_application(
        MEET,
        "[Desktop Entry]\nType=Application\nName=Video Meet\nExec=flatpak run org.example.Meet\nX-Flatpak=org.example.Meet\n",
    );
    let harness = sandbox.harness(privacy());
    World {
        portal,
        harness,
        _rt: rt,
        _bus: bus,
    }
}

/// `html` without its `<style>` blocks: the stylesheet's comments name System Settings too.
fn without_styles(html: &str) -> String {
    let mut rest = html;
    let mut kept = String::with_capacity(html.len());
    while let Some(start) = rest.find("<style") {
        kept.push_str(&rest[..start]);
        rest = rest[start..]
            .find("</style>")
            .map_or("", |end| &rest[start + end + "</style>".len()..]);
    }
    kept.push_str(rest);
    kept
}

fn listed(w: &mut World) {
    settle(&mut w.harness, "the list", |h| h.count(GROUP) == 1);
}

/// The programs above are on the page too. They load on their own and push the rows below down
/// when they come, so a click aimed before then can land on another row.
fn steady(w: &mut World) {
    listed(w);
    settle(&mut w.harness, "the programs above", |h| {
        h.count("[data-privacy=\"list\"]") == 1
    });
}

#[test]
fn the_apps_the_portal_remembers_are_named_by_the_id_it_proved_and_say_when_and_for_how_long() {
    let mut w = world("screen-sharing-list", &remembered());
    listed(&mut w);
    let html = w.harness.html();
    for text in [
        "Screen sharing",
        "Video Meet",
        "Allowed 3 days ago, until you remove it",
        CALL,
        "Allowed 2 hours ago, while it is open",
        GRABBER,
        "Allowed yesterday, until you remove it",
    ] {
        assert!(html.contains(text), "{text} in {html}");
    }
    assert!(
        !without_styles(&html).contains("System Settings"),
        "the portal's own name for an app is not shown: {html}"
    );
    let at = |text: &str| html.find(text).unwrap_or(usize::MAX);
    assert!(
        at(CALL) < at(GRABBER) && at(GRABBER) < at("Video Meet"),
        "the app allowed last comes first"
    );
    assert_eq!(w.harness.count("[aria-label^=\"Remove \"]"), 3);
    // The programs above load on their own; they come too.
    settle(&mut w.harness, "the programs above", |h| {
        h.count("[data-privacy=\"list\"]") == 1
    });
}

#[test]
fn remove_takes_the_app_back_and_its_row_goes() {
    let mut w = world("screen-sharing-remove", &remembered());
    steady(&mut w);
    let portal = w.portal.as_ref().expect("a portal");
    assert!(portal.revoked().is_empty());
    click(&mut w.harness, REMOVE_MEET);
    settle(&mut w.harness, "the row to go", |h| {
        h.count(REMOVE_MEET) == 0
    });
    let portal = w.portal.as_ref().expect("a portal");
    assert_eq!(portal.revoked(), [MEET]);
    assert_eq!(
        w.harness.count("[aria-label^=\"Remove \"]"),
        2,
        "the other two stay"
    );
    assert!(w.harness.html().contains(CALL));
}

#[test]
fn a_refusal_is_told_in_plain_words_and_the_row_stays() {
    let mut w = world("screen-sharing-refused", &remembered());
    steady(&mut w);
    w.portal.as_ref().expect("a portal").refuse_next();
    click(&mut w.harness, REMOVE_MEET);
    settle(&mut w.harness, "the toast", |h| h.count(".ds-toast") == 1);
    let toast = w.harness.text_of(".ds-toast").unwrap_or_default();
    assert!(
        toast.contains("Screen sharing could not be changed"),
        "{toast}"
    );
    assert!(w.portal.as_ref().expect("a portal").revoked().is_empty());
    assert_eq!(w.harness.count(REMOVE_MEET), 1, "the app is still listed");
    // A second press works: the first refusal did not leave the list stuck. The toast sits over
    // the last row at this window size, so the press waits for it to go.
    settle(&mut w.harness, "the toast to go", |h| {
        h.count(".ds-toast") == 0
    });
    click(&mut w.harness, REMOVE_MEET);
    settle(&mut w.harness, "the row to go", |h| {
        h.count(REMOVE_MEET) == 0
    });
}
