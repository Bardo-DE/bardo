use crate::support::{
    KEY_REMAP_UID, MOUSE_GESTURES_CGROUP, MOUSE_GESTURES_UID, USER, key_remap, listed,
    mouse_gestures,
};
use caller_identity_core::{Proof, Uid};
use input_broker_daemon::identity::{ActiveUid, Caller};
use input_broker_daemon::policy::{Refusal, authorise};
use input_broker_wire::Kind;

const MOUSE_GESTURES_KINDS: [Kind; 3] = [
    Kind::MouseGesturesTouchpad,
    Kind::MouseGesturesWheel,
    Kind::MouseGesturesKeys,
];
const KEY_REMAP_KINDS: [Kind; 2] = [Kind::KeyRemapKeyboard, Kind::KeyRemapPointer];

fn seat(uid: Option<Uid>) -> ActiveUid {
    ActiveUid(uid)
}

#[test]
fn the_genuine_programs_get_their_own_devices_and_only_those() {
    for kind in MOUSE_GESTURES_KINDS {
        assert_eq!(
            authorise(kind, &mouse_gestures(), seat(Some(USER))),
            Ok(()),
            "{kind:?}"
        );
        assert!(
            authorise(kind, &key_remap(), seat(Some(USER))).is_err(),
            "key_remap asking for {kind:?}"
        );
    }
    for kind in KEY_REMAP_KINDS {
        assert_eq!(
            authorise(kind, &key_remap(), seat(Some(USER))),
            Ok(()),
            "{kind:?}"
        );
        assert!(
            authorise(kind, &mouse_gestures(), seat(Some(USER))).is_err(),
            "mouse_gestures asking for {kind:?}"
        );
    }
}

#[test]
fn both_are_served_before_anyone_logs_in() {
    assert_eq!(
        authorise(Kind::KeyRemapKeyboard, &key_remap(), seat(None)),
        Ok(())
    );
    assert_eq!(
        authorise(Kind::MouseGesturesWheel, &mouse_gestures(), seat(None)),
        Ok(())
    );
}

#[test]
fn refusals_table() {
    use Refusal::*;
    let user_mouse_gestures =
        "/user.slice/user-1000.slice/user@1000.service/app.slice/bardo-mouse-gestures.service";
    let user_key_remap =
        "/user.slice/user-1000.slice/user@1000.service/app.slice/bardo-key-remap.service";
    let table: Vec<(&str, Kind, Caller, ActiveUid, Refusal)> = vec![
        (
            "not in the table",
            Kind::MouseGesturesWheel,
            Caller {
                credential: None,
                ..mouse_gestures()
            },
            seat(Some(USER)),
            Unlisted,
        ),
        (
            "another role: the shell",
            Kind::MouseGesturesWheel,
            listed(
                "shell",
                Proof::Full,
                MOUSE_GESTURES_UID,
                MOUSE_GESTURES_CGROUP,
            ),
            seat(Some(USER)),
            WrongRole {
                have: "shell".to_owned(),
            },
        ),
        (
            "mouse_gestures asks for key_remap's keyboard",
            Kind::KeyRemapKeyboard,
            mouse_gestures(),
            seat(Some(USER)),
            WrongRole {
                have: "pad_daemon".to_owned(),
            },
        ),
        (
            "key_remap asks for mouse_gestures's keys",
            Kind::MouseGesturesKeys,
            key_remap(),
            seat(Some(USER)),
            WrongRole {
                have: "keymap_daemon".to_owned(),
            },
        ),
        (
            "mouse_gestures proven by its control group only",
            Kind::MouseGesturesTouchpad,
            listed(
                "pad_daemon",
                Proof::Cgroup,
                MOUSE_GESTURES_UID,
                MOUSE_GESTURES_CGROUP,
            ),
            seat(Some(USER)),
            TooWeak,
        ),
        (
            "key_remap proven by its control group only",
            Kind::KeyRemapKeyboard,
            listed(
                "keymap_daemon",
                Proof::Cgroup,
                KEY_REMAP_UID,
                "/system.slice/bardo-key-remap.service",
            ),
            seat(Some(USER)),
            TooWeak,
        ),
        (
            "mouse_gestures running as the logged in user",
            Kind::MouseGesturesWheel,
            listed("pad_daemon", Proof::Full, USER, MOUSE_GESTURES_CGROUP),
            seat(Some(USER)),
            WrongUser,
        ),
        (
            "mouse_gestures as a user unit of the person",
            Kind::MouseGesturesWheel,
            listed(
                "pad_daemon",
                Proof::Full,
                MOUSE_GESTURES_UID,
                user_mouse_gestures,
            ),
            seat(Some(USER)),
            WrongPlace,
        ),
        (
            "mouse_gestures in a lookalike system unit",
            Kind::MouseGesturesWheel,
            listed(
                "pad_daemon",
                Proof::Full,
                MOUSE_GESTURES_UID,
                "/system.slice/bardo-mouse-gestures.service/x",
            ),
            seat(Some(USER)),
            WrongPlace,
        ),
        (
            "mouse_gestures in key_remap's unit",
            Kind::MouseGesturesWheel,
            listed(
                "pad_daemon",
                Proof::Full,
                MOUSE_GESTURES_UID,
                "/system.slice/bardo-key-remap.service",
            ),
            seat(Some(USER)),
            WrongPlace,
        ),
        (
            "key_remap running as the logged in user",
            Kind::KeyRemapKeyboard,
            listed(
                "keymap_daemon",
                Proof::Full,
                USER,
                "/system.slice/bardo-key-remap.service",
            ),
            seat(Some(USER)),
            WrongUser,
        ),
        (
            "key_remap as a user unit of the person",
            Kind::KeyRemapKeyboard,
            listed("keymap_daemon", Proof::Full, KEY_REMAP_UID, user_key_remap),
            seat(Some(USER)),
            WrongPlace,
        ),
        (
            "key_remap in a lookalike system unit",
            Kind::KeyRemapPointer,
            listed(
                "keymap_daemon",
                Proof::Full,
                KEY_REMAP_UID,
                "/system.slice/bardo-key-remap.service/x",
            ),
            seat(Some(USER)),
            WrongPlace,
        ),
    ];
    for (name, kind, caller, active, want) in table {
        assert_eq!(authorise(kind, &caller, active), Err(want), "{name}");
    }
}

/// NEW-PI-4: every proof below the unit's own is refused for every kind, in the right place and
/// as the right user; only the unit's proof is enough.
#[test]
fn only_a_proven_unit_gets_devices_whatever_the_place_is() {
    let table: &[(&str, Proof, Result<(), Refusal>)] = &[
        (
            "its control group only",
            Proof::Cgroup,
            Err(Refusal::TooWeak),
        ),
        (
            "its control group and its image",
            Proof::Exe,
            Err(Refusal::TooWeak),
        ),
        ("its unit", Proof::Full, Ok(())),
    ];
    for (name, proof, want) in table {
        let pad = listed(
            "pad_daemon",
            *proof,
            MOUSE_GESTURES_UID,
            MOUSE_GESTURES_CGROUP,
        );
        let key = listed(
            "keymap_daemon",
            *proof,
            KEY_REMAP_UID,
            "/system.slice/bardo-key-remap.service",
        );
        for kind in MOUSE_GESTURES_KINDS {
            assert_eq!(
                authorise(kind, &pad, seat(Some(USER))),
                *want,
                "mouse_gestures, {name}, {kind:?}"
            );
        }
        for kind in KEY_REMAP_KINDS {
            assert_eq!(
                authorise(kind, &key, seat(Some(USER))),
                *want,
                "key_remap, {name}, {kind:?}"
            );
        }
    }
}
