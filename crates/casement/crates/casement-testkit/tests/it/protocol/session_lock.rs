//! `session_lock` (P0-H3).

use casement_core::OutputName;
use casement_core::geometry::PhysicalSize;
use casement_core::input::{DeviceClass, KeyCode};
use casement_core::lock::LockState;
use casement_core::policy::ConnectionRole;
use casement_protocols::vocab::PressEdge;
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::surface::lock::Locker;
use casement_testclient::{Pattern, TestClient, shm_buffer};
use casement_testkit::{Fixture, FixtureSpec, FrameCount, ShotTarget, TestOrigin};
use std::time::Duration;

use crate::protocol::consent::{asked_world, kept, program};
use crate::protocol::text_input::Typing;
use crate::protocol::xdg_shell::{map_toplevel, settle};
use casement_core::Event;
use casement_core::privilege::{UnixSeconds, Verdict};
use casement_protocols::vocab::{Answer, Power};
use casement_testclient::ime::input_method::InputMethodEvent;
use casement_testclient::surface::toplevel::WmBase;
use casement_testkit::ASKED_PROGRAM;

/// An opaque black pixel.
const BLACK: [u8; 4] = [0, 0, 0, 255];

/// How far from where it rests the cursor reaches.
const CURSOR_REACH: u32 = 32;

/// Where the cursor rests: the centre of the 1920x1080 output, where the pointer starts.
const CURSOR_AT: (u32, u32) = (960, 540);

/// The evdev code of the `a` key.
const KEY_A: KeyCode = KeyCode(30);

/// How many `key` events `client`'s keyboards received.
fn keys_received(client: &TestClient) -> usize {
    client
        .transcript()
        .0
        .iter()
        .filter(|line| line.object.starts_with("wl_keyboard#") && line.event == "key")
        .count()
}

/// Whether `client`'s keyboard has been told it left its focus.
fn keyboard_left(client: &TestClient) -> bool {
    client
        .transcript()
        .0
        .iter()
        .any(|line| line.object.starts_with("wl_keyboard#") && line.event == "leave")
}

/// While locked, keys and pointer reach only the lock surface.
#[test]
fn lock_blocks_input_to_windows() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let keyboard_device = fixture.input().plug(
        DeviceClass::Keyboard,
        TestOrigin::Physical(DeviceClass::Keyboard),
    );
    // One client holds the keyboard through an exclusive layer, as a launcher does.
    let shell = fixture.connect(ConnectionRole::Shell);
    let locker_client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let _shell_keyboard = Keyboard::bind(fixture.client(shell)).expect("keyboard");
    let _locker_keyboard = Keyboard::bind(fixture.client(locker_client)).expect("keyboard");
    let launcher = LayerSurface::create(
        fixture.client(shell),
        &LayerAsk {
            level: Layer::Overlay,
            namespace: "launcher",
            anchor: Anchor::empty(),
            exclusive_zone: 0,
            keyboard: KeyboardInteractivity::Exclusive,
            size: (400, 300),
        },
    )
    .expect("layer");
    fixture.pump();
    let _ = launcher;
    fixture
        .input()
        .key(keyboard_device, KEY_A, PressEdge::Press);
    fixture
        .input()
        .key(keyboard_device, KEY_A, PressEdge::Release);
    assert!(
        keys_received(fixture.client(shell)) >= 2,
        "before the lock the launcher has the keys"
    );

    // The lock arrives and covers the output.
    let locker = Locker::request(fixture.client(locker_client)).expect("locker");
    fixture.pump();
    let cover = locker
        .cover(fixture.client(locker_client))
        .expect("lock surface");
    fixture.pump();
    assert!(cover.ack(fixture.client(locker_client)));
    let size = PhysicalSize { w: 1920, h: 1080 };
    let buffer = shm_buffer(
        fixture.client(locker_client),
        size,
        Pattern::Solid(0xff00_0000),
    );
    cover.show(fixture.client(locker_client), &buffer);
    fixture.pump();
    assert!(locker.is_locked());
    assert!(
        keyboard_left(fixture.client(shell)),
        "the launcher lost the keyboard to the lock"
    );

    let before = (
        keys_received(fixture.client(shell)),
        keys_received(fixture.client(locker_client)),
    );
    fixture
        .input()
        .key(keyboard_device, KEY_A, PressEdge::Press);
    fixture
        .input()
        .key(keyboard_device, KEY_A, PressEdge::Release);
    let after = (
        keys_received(fixture.client(shell)),
        keys_received(fixture.client(locker_client)),
    );
    assert_eq!(
        after.0, before.0,
        "no key reaches the session behind the lock"
    );
    assert!(after.1 > before.1, "the lock surface hears the keys");
}

/// Killing the locker keeps every output covered; nothing of the session is drawn.
#[test]
fn a_dead_locker_leaves_the_lock_cover() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let app = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let base = WmBase::bind(fixture.client(app), 6);
    let _window = map_toplevel(&mut fixture, app, &base, (400, 300));
    settle(&mut fixture);
    // Before the lock the window is on the output: some pixel is not black.
    let output = OutputName::parse("HEADLESS-1").expect("a valid output name");
    fixture.frames(&output, FrameCount(2));
    let before = fixture.shot(ShotTarget::Output(output.clone()));
    assert!(
        before.pixels().any(|pixel| pixel.0 != BLACK),
        "the window is drawn before the lock"
    );

    // A locker covers the output, then dies.
    let owner = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let locker = Locker::request(fixture.client(owner)).expect("a lock");
    fixture.pump();
    let cover = locker.cover(fixture.client(owner)).expect("a lock surface");
    fixture.pump();
    assert!(cover.ack(fixture.client(owner)));
    let size = PhysicalSize { w: 1920, h: 1080 };
    let buffer = shm_buffer(fixture.client(owner), size, Pattern::CoordinateEncoding);
    cover.show(fixture.client(owner), &buffer);
    fixture.pump();
    assert!(locker.is_locked(), "the session is locked");
    fixture.disconnect(owner);
    fixture.pump();
    fixture.advance(Duration::from_secs(2));
    fixture.frames(&output, FrameCount(2));

    // The compositor's own cover stands in: the session is still locked, and nothing of the
    // window (or of the dead locker's lock surface) is drawn.
    assert_eq!(
        fixture.model().windows().len(),
        1,
        "the window is still there, behind the cover"
    );
    let shot = fixture.shot(ShotTarget::Output(output));
    let leaked = shot
        .enumerate_pixels()
        .filter(|(x, y, _)| {
            x.abs_diff(CURSOR_AT.0) >= CURSOR_REACH || y.abs_diff(CURSOR_AT.1) >= CURSOR_REACH
        })
        .filter(|(_, _, pixel)| pixel.0 != BLACK)
        .count();
    assert_eq!(leaked, 0, "the cover left {leaked} pixels showing");
}

/// `id` locks the session with a lock surface that shows content on the first output, and the
/// session is locked when this returns.
fn lock_with_content(fixture: &mut Fixture, id: casement_testkit::ClientId) -> Locker {
    let locker = Locker::request(fixture.client(id)).expect("a lock");
    fixture.pump();
    let cover = locker.cover(fixture.client(id)).expect("a lock surface");
    fixture.pump();
    assert!(cover.ack(fixture.client(id)));
    let size = PhysicalSize { w: 1920, h: 1080 };
    let buffer = shm_buffer(fixture.client(id), size, Pattern::Solid(0xff00_0000));
    cover.show(fixture.client(id), &buffer);
    fixture.pump();
    assert!(locker.is_locked(), "the session is locked");
    locker
}

/// Locked arrives only after each output shows a lock surface or a cover.
#[test]
fn locked_is_sent_once_every_output_is_covered() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let locker = Locker::request(fixture.client(id)).expect("locker");
    fixture.pump();
    assert!(
        !locker.is_locked(),
        "nothing is locked before the output is covered"
    );
    assert!(!locker.is_finished());
    let cover = locker.cover(fixture.client(id)).expect("lock surface");
    fixture.pump();
    assert!(
        !locker.is_locked(),
        "a lock surface without content does not cover the output"
    );
    assert!(cover.ack(fixture.client(id)));
    let size = PhysicalSize { w: 1920, h: 1080 };
    let buffer = shm_buffer(fixture.client(id), size, Pattern::Solid(0xff00_0000));
    cover.show(fixture.client(id), &buffer);
    fixture.pump();
    assert!(locker.is_locked(), "locked arrives with the first content");
}

/// Covers the first output with a lock surface that shows content, for a locker that was
/// requested already.
fn cover_with_content(fixture: &mut Fixture, id: casement_testkit::ClientId, locker: &Locker) {
    let cover = locker.cover(fixture.client(id)).expect("a lock surface");
    fixture.pump();
    assert!(cover.ack(fixture.client(id)));
    let size = PhysicalSize { w: 1920, h: 1080 };
    let buffer = shm_buffer(fixture.client(id), size, Pattern::Solid(0xff00_0000));
    cover.show(fixture.client(id), &buffer);
    fixture.pump();
}

/// Locking is a power: a program that is asked gets neither `locked` nor `finished` while the
/// question is open (and the session stays open however long the wait), the shell is asked once,
/// and on Allow the normal lock flow goes on.
#[test]
fn a_lock_waits_for_the_answer_and_goes_on_when_allowed() {
    let (mut fixture, shell, client) = asked_world();
    let locker = Locker::request(fixture.client(client)).expect("a lock");
    fixture.pump();
    assert_eq!(shell.asked().len(), 1, "the shell is asked once");
    assert_eq!(shell.asked()[0].power, Power::Lock);
    fixture.advance(Duration::from_secs(2));
    assert!(!locker.is_locked() && !locker.is_finished(), "it waits");
    assert_eq!(fixture.model().lock(), &LockState::Unlocked);

    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    assert!(matches!(fixture.model().lock(), LockState::Locking { .. }));
    cover_with_content(&mut fixture, client, &locker);
    assert!(locker.is_locked(), "the lock goes on once allowed");
    assert!(!locker.is_finished());
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Lock),
        Some(Verdict::Allow)
    );
}

/// Don't Allow: the locker is told `finished` and the session stays open.
#[test]
fn a_refused_lock_is_finished_and_the_session_stays_open() {
    let (mut fixture, shell, client) = asked_world();
    let locker = Locker::request(fixture.client(client)).expect("a lock");
    fixture.pump();
    assert!(!locker.is_finished());
    shell.answer(&mut fixture, shell.only_ask(), Answer::Deny);
    assert!(locker.is_finished(), "the refusal is `finished`");
    assert!(!locker.is_locked());
    assert_eq!(fixture.model().lock(), &LockState::Unlocked);
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Lock),
        Some(Verdict::Deny)
    );
}

/// Taking the power back never ends a lock that already holds the session: the covers stay up
/// and the locker can still unlock.
#[test]
fn taking_the_power_back_does_not_end_a_lock_in_force() {
    let (mut fixture, shell, client) = asked_world();
    let locker = Locker::request(fixture.client(client)).expect("a lock");
    fixture.pump();
    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    cover_with_content(&mut fixture, client, &locker);
    assert!(locker.is_locked());

    fixture.feed_events(vec![Event::PrivilegeEdited {
        edit: casement_core::privilege::Edit::Deny(program(ASKED_PROGRAM), Power::Lock),
        at: UnixSeconds(2),
    }]);
    fixture.advance(Duration::from_secs(1));
    assert!(!locker.is_finished(), "the lock is not ended");
    assert!(matches!(fixture.model().lock(), LockState::Locked { .. }));

    locker.unlock(fixture.client(client));
    fixture.pump();
    assert_eq!(
        fixture.model().lock(),
        &LockState::Unlocked,
        "only the locker unlocks, and it still can"
    );
}

/// The screensaver locks while an input method holds the keyboard (fcitx5 on the trial login of
/// 2026-10-20): the lock screen's password field enables text input as any field does. A real
/// key must reach the lock surface itself, not the input method: the method can only answer
/// through its virtual keyboard, which the lock refuses, so the field would hear nothing. The
/// method is not activated on the lock screen and never sees the key, as the Mac turns input
/// methods off for a password field.
#[test]
fn a_key_on_the_lock_screen_reaches_it_past_a_running_input_method() {
    let mut typing = Typing::start();
    typing.enable_and_grab();
    let activations = |typing: &Typing| {
        typing
            .ime
            .method_events()
            .iter()
            .filter(|line| line.event == InputMethodEvent::Activate)
            .count()
    };
    let before = activations(&typing);
    let client = typing.client;
    let _locker = lock_with_content(&mut typing.fixture, client);
    typing.pump();
    typing
        .ime
        .enable()
        .expect("the password field enables text input");
    typing.pump();
    typing.pump();
    assert_eq!(
        activations(&typing),
        before,
        "the method is not activated on the lock screen"
    );
    let heard = typing.ime.document_keys().len();
    let keyboard = typing.keyboard;
    typing
        .fixture
        .input()
        .key(keyboard, KEY_A, PressEdge::Press);
    typing
        .fixture
        .input()
        .key(keyboard, KEY_A, PressEdge::Release);
    typing.pump();
    typing.pump();
    assert!(
        typing
            .ime
            .method_events()
            .iter()
            .all(|line| !matches!(line.event, InputMethodEvent::GrabKey { .. })),
        "the method never sees a key typed on the lock screen"
    );
    let keys: Vec<_> = typing.ime.document_keys()[heard..]
        .iter()
        .map(|key| (key.key, key.state))
        .collect();
    assert_eq!(
        keys,
        vec![(30, 1), (30, 0)],
        "the lock surface hears the key"
    );
}

/// The lock is another program than the app and its input method (sill's lock, fcitx5 and the
/// app the person was typing in): a key typed on the lock screen reaches the locker's keyboard
/// and none reaches the app behind it, whether the method keeps its grab or lets it go when it
/// is deactivated (fcitx5 lets it go at the `done` after `deactivate`).
#[test]
fn a_key_on_the_lock_screen_reaches_the_locker_not_the_app_under_an_input_method() {
    for release in [GrabAfterLock::Kept, GrabAfterLock::Released] {
        let mut typing = Typing::start();
        typing.enable_and_grab();
        let locker_client = typing.fixture.connect(ConnectionRole::Public);
        typing.fixture.pump();
        let _locker_keyboard =
            Keyboard::bind(typing.fixture.client(locker_client)).expect("keyboard");
        let _locker = lock_with_content(&mut typing.fixture, locker_client);
        typing.pump();
        if release == GrabAfterLock::Released {
            typing
                .ime
                .release_grab()
                .expect("the method lets the grab go");
        }
        typing.pump();
        let app = typing.client;
        let before = (
            keys_received(typing.fixture.client(app)),
            keys_received(typing.fixture.client(locker_client)),
        );
        let keyboard = typing.keyboard;
        typing
            .fixture
            .input()
            .key(keyboard, KEY_A, PressEdge::Press);
        typing
            .fixture
            .input()
            .key(keyboard, KEY_A, PressEdge::Release);
        typing.pump();
        typing.pump();
        let after = (
            keys_received(typing.fixture.client(app)),
            keys_received(typing.fixture.client(locker_client)),
        );
        assert_eq!(
            after.0, before.0,
            "{release:?}: no key reaches the app behind the lock"
        );
        assert_eq!(
            after.1,
            before.1 + 2,
            "{release:?}: the lock hears the press and the release"
        );
    }
}

/// What the input method does with its keyboard grab once the lock deactivated it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GrabAfterLock {
    /// It keeps it.
    Kept,
    /// It lets it go, as fcitx5 does.
    Released,
}
