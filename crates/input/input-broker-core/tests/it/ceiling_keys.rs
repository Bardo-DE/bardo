//! The mouse gestures daemon's keyboard: shortcuts yes, typing no.

use crate::{key, sync};
use input_broker_core::ceiling::{BadCapsError, Denial, Filter, Route, plan};
use input_broker_core::limits::MAX_KEYS_DOWN;
use input_broker_wire::codes::*;
use input_broker_wire::limits::MAX_KEY_CODES;
use input_broker_wire::{Create, Event};
use key_types::KeyCode;

const A: u16 = 30;
const ENTER: u16 = 28;
const LEFT: u16 = 105;
const F5: u16 = 63;
const SHIFT: u16 = KeyCode::LEFTSHIFT.code();

fn filter(codes: &[u16]) -> Filter {
    let planned = plan(&Create::keys(codes.to_vec())).expect("within the ceiling");
    let Route::Proxied(filter) = planned.route else {
        panic!("proxied");
    };
    filter
}

fn verdict(f: &mut Filter, event: Event) -> Result<(), Denial> {
    f.check(event).map(|_| ())
}

#[test]
fn what_mouse_gestures_writes_for_alt_left_passes() {
    let mut f = filter(&[KeyCode::LEFTALT.code(), LEFT]);
    let out = f.apply(&[
        key(KeyCode::LEFTALT.code(), 1),
        key(LEFT, 1),
        sync(),
        key(LEFT, 0),
        key(KeyCode::LEFTALT.code(), 0),
        sync(),
    ]);
    assert!(out.denied.is_empty(), "{out:?}");
    assert_eq!(out.pass.len(), 6);
}

#[test]
fn typing_is_refused_and_shortcuts_are_not() {
    let mut f = filter(&[
        A,
        ENTER,
        57,
        14,
        111,
        1,
        2,
        F5,
        LEFT,
        158,
        SHIFT,
        KeyCode::LEFTCTRL.code(),
        KeyCode::LEFTALT.code(),
        KeyCode::LEFTMETA.code(),
    ]);
    // Without a chord modifier only the free keys go down.
    assert_eq!(verdict(&mut f, key(A, 1)), Err(Denial::NeedsChord));
    assert_eq!(verdict(&mut f, key(ENTER, 1)), Err(Denial::NeedsChord));
    assert_eq!(
        verdict(&mut f, key(57, 1)),
        Err(Denial::NeedsChord),
        "space"
    );
    assert_eq!(
        verdict(&mut f, key(14, 1)),
        Err(Denial::NeedsChord),
        "backspace"
    );
    assert_eq!(
        verdict(&mut f, key(111, 1)),
        Err(Denial::NeedsChord),
        "delete"
    );
    assert_eq!(
        verdict(&mut f, key(1, 1)),
        Err(Denial::NeedsChord),
        "escape"
    );
    assert_eq!(verdict(&mut f, key(2, 1)), Err(Denial::NeedsChord), "digit");
    assert_eq!(verdict(&mut f, key(F5, 1)), Ok(()));
    assert_eq!(verdict(&mut f, key(LEFT, 1)), Ok(()));
    assert_eq!(verdict(&mut f, key(158, 1)), Ok(()), "back");
    // Shift is not a chord.
    assert_eq!(verdict(&mut f, key(SHIFT, 1)), Ok(()));
    assert_eq!(verdict(&mut f, key(A, 1)), Err(Denial::NeedsChord));
    assert_eq!(verdict(&mut f, key(SHIFT, 0)), Ok(()));
    // Each chord modifier lets a letter through, and only while it is down.
    for chord in [
        KeyCode::LEFTCTRL.code(),
        KeyCode::LEFTALT.code(),
        KeyCode::LEFTMETA.code(),
    ] {
        assert_eq!(verdict(&mut f, key(chord, 1)), Ok(()));
        assert_eq!(verdict(&mut f, key(A, 1)), Ok(()), "{chord}");
        assert_eq!(verdict(&mut f, key(A, 0)), Ok(()));
        assert_eq!(verdict(&mut f, key(chord, 0)), Ok(()));
        assert_eq!(
            verdict(&mut f, key(A, 1)),
            Err(Denial::NeedsChord),
            "{chord} released"
        );
    }
}

#[test]
fn a_key_already_down_stays_releasable_after_its_modifier_lifts() {
    let mut f = filter(&[KeyCode::LEFTCTRL.code(), A]);
    assert_eq!(verdict(&mut f, key(KeyCode::LEFTCTRL.code(), 1)), Ok(()));
    assert_eq!(verdict(&mut f, key(A, 1)), Ok(()));
    assert_eq!(verdict(&mut f, key(KeyCode::LEFTCTRL.code(), 0)), Ok(()));
    assert_eq!(
        verdict(&mut f, key(A, 0)),
        Ok(()),
        "release is never refused"
    );
}

#[test]
fn repeat_other_values_and_other_types_are_refused() {
    let mut f = filter(&[KeyCode::LEFTCTRL.code(), LEFT]);
    assert_eq!(
        verdict(&mut f, key(LEFT, 2)),
        Err(Denial::BadValue),
        "auto-repeat"
    );
    assert_eq!(verdict(&mut f, key(LEFT, -1)), Err(Denial::BadValue));
    assert_eq!(
        verdict(&mut f, key(A, 1)),
        Err(Denial::NotAllowed),
        "not registered"
    );
    assert_eq!(
        verdict(&mut f, Event::new(EV_REL, REL_X, 1)),
        Err(Denial::NotAllowed)
    );
    assert_eq!(
        verdict(&mut f, Event::new(EV_ABS, ABS_X, 1)),
        Err(Denial::NotAllowed)
    );
    assert_eq!(
        verdict(&mut f, Event::new(EV_LED, LED_CAPSL, 1)),
        Err(Denial::NotAllowed)
    );
    assert_eq!(
        verdict(&mut f, Event::new(0x14, 0, 1)),
        Err(Denial::NotAllowed),
        "EV_REP"
    );
}

#[test]
fn at_most_eight_keys_are_down_at_once() {
    let free: Vec<u16> = (59..=66).chain([87, 88, 102]).collect();
    let mut f = filter(&free);
    for &code in free.iter().take(MAX_KEYS_DOWN) {
        assert_eq!(verdict(&mut f, key(code, 1)), Ok(()));
    }
    assert_eq!(
        verdict(&mut f, key(free[MAX_KEYS_DOWN], 1)),
        Err(Denial::TooManyKeys)
    );
    assert_eq!(verdict(&mut f, key(free[0], 0)), Ok(()));
    assert_eq!(verdict(&mut f, key(free[MAX_KEYS_DOWN], 1)), Ok(()));
}

#[test]
fn requests_outside_the_name_table_are_refused() {
    let table: &[(&str, Vec<u16>, BadCapsError)] = &[
        ("no keys", vec![], BadCapsError::NoKeys),
        (
            "a right-hand control",
            vec![97],
            BadCapsError::UnknownKey(97),
        ),
        ("key zero", vec![0], BadCapsError::UnknownKey(0)),
        ("a button", vec![0x110], BadCapsError::UnknownKey(0x110)),
        (
            "one good one bad",
            vec![A, 9999],
            BadCapsError::UnknownKey(9999),
        ),
        (
            "over the cap",
            vec![A; MAX_KEY_CODES + 1],
            BadCapsError::TooManyKeys,
        ),
    ];
    for (name, codes, want) in table {
        let got = plan(&Create::keys(codes.clone())).err();
        assert_eq!(got, Some(*want), "{name}");
    }
}

#[test]
fn the_device_registers_exactly_the_requested_keys_once() {
    let spec = plan(&Create::keys(vec![
        LEFT,
        KeyCode::LEFTALT.code(),
        LEFT,
        106,
    ]))
    .expect("plans")
    .spec;
    assert_eq!(spec.name, "mouse-gestures keys");
    assert_eq!((spec.vendor, spec.product), (0x0001, 0x0003));
    assert_eq!(spec.keys, [KeyCode::LEFTALT.code(), LEFT, 106]);
    assert!(spec.rels.is_empty() && spec.abs.is_empty() && spec.leds.is_empty());
    assert_eq!(spec.event_types(), [EV_KEY]);
}
