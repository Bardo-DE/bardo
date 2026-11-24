//! The note under "Models on this computer" when the assistant has not found llama.cpp, over a
//! fake inferd that lists the engines and a fake service manager that writes down each restart,
//! both on a private bus: the line, the command for the distro, the releases page for a release
//! with no package, nothing when it is found or the assistant is not running, and the button that
//! asks first and restarts once.

use crate::support::Sandbox;
use crate::support::drive::{click, scroll_into_view};
use crate::support::drive::{let_threads_run, settle_on_threads};
use crate::support::engines::{Fake, serve};
use base_testkit::PrivateBus;
use settings_app_model::PageSlug;
use settings_app_ui::Opening;
use ui_harness::Harness;

const NOTE: &str = "[data-engine-note]";
const COMMAND: &str = "[data-engine-note] .ds-command-line-text";
const START: &str = ".engine-note-start .ds-button";
const ASKED: &str = ".ds-alert";
/// What the note says over the install command.
const SETUP_LINE: &str = "To run models on this computer, install llama.cpp.";

/// What the button that makes the assistant look again says.
const START_USING: &str = "Start using it";

const CANCEL: &str = ".ds-alert .ds-button[data-answers=\"escape\"]";

const FEDORA: &str = "ID=fedora\nVERSION_ID=44\n";

fn intelligence() -> Opening {
    Opening {
        page: Some(PageSlug("intelligence".to_owned())),
        query: String::new(),
    }
}

struct World {
    fake: Fake,
    harness: Harness,
    _rt: tokio::runtime::Runtime,
    _bus: PrivateBus,
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("a runtime for the fakes")
}

/// The Intelligence page on a computer of the distro `os_release` describes, with inferd (llama.cpp's
/// program there or not) and the service manager on the bus.
fn world(name: &str, os_release: &str, found: bool) -> World {
    let rt = runtime();
    let bus = PrivateBus::start();
    let sandbox = Sandbox::new(name).on_bus(bus.address());
    sandbox.write_os_release(os_release);
    let fake = rt.block_on(serve(&bus, found));
    let harness = sandbox.harness(intelligence());
    World {
        fake,
        harness,
        _rt: rt,
        _bus: bus,
    }
}

#[test]
fn a_missing_llama_cpp_shows_the_line_the_command_and_the_button_and_nothing_about_the_rest() {
    let mut w = world("engines-missing", FEDORA, false);
    settle_on_threads(&mut w.harness, "the note", |h| h.count(NOTE) == 1);
    let note = w.harness.text_of(NOTE).unwrap_or_default();
    assert!(note.contains(SETUP_LINE), "{note}");
    assert_eq!(
        w.harness.text_of(COMMAND).as_deref().map(str::trim),
        Some("sudo dnf install llama-cpp")
    );
    assert_eq!(w.harness.count(START), 1);
    assert!(
        w.harness
            .text_of(START)
            .unwrap_or_default()
            .contains(START_USING),
        "the button says what it does"
    );
    for quiet in ["vLLM", "Kokoro", "Speech"] {
        assert!(!note.contains(quiet), "{quiet} stays quiet: {note}");
    }
    assert!(w.fake.restarts().is_empty(), "looking restarts nothing");
}

#[test]
fn start_using_it_asks_first_and_cancel_restarts_nothing() {
    let mut w = world("engines-cancel", FEDORA, false);
    settle_on_threads(&mut w.harness, "the note", |h| h.count(NOTE) == 1);
    scroll_into_view(&mut w.harness, START);
    click(&mut w.harness, START);
    settle_on_threads(&mut w.harness, "the question", |h| h.count(ASKED) == 1);
    let words = w.harness.text_of(ASKED).unwrap_or_default();
    assert!(words.contains("conversation that is open stops"), "{words}");
    for developer in ["systemd", "D-Bus", "inferd"] {
        assert!(!words.contains(developer), "{developer}: {words}");
    }
    assert!(w.fake.restarts().is_empty(), "asking restarts nothing");
    click(&mut w.harness, CANCEL);
    settle_on_threads(&mut w.harness, "the question to go", |h| {
        h.count(ASKED) == 0
    });
    // Lets whatever the window would still send reach the fakes.
    let_threads_run(&mut w.harness, 20);
    assert!(w.fake.restarts().is_empty(), "Cancel restarts nothing");
    assert_eq!(w.harness.count(NOTE), 1, "the note stays");
}
