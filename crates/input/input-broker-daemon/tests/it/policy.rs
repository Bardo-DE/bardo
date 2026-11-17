use crate::support::{
    KEY_REMAP_UID, PALMREST_CGROUP, PALMREST_UID, USER, key_remap, listed, palmrest,
};
use caller_identity_core::{Proof, Uid};
use input_broker_daemon::identity::{ActiveUid, Caller};
use input_broker_daemon::policy::{Refusal, authorise};
use input_broker_wire::Kind;

const PALMREST_KINDS: [Kind; 3] = [
    Kind::PalmrestTouchpad,
    Kind::PalmrestWheel,
    Kind::PalmrestKeys,
];
const KEY_REMAP_KINDS: [Kind; 2] = [Kind::KeyRemapKeyboard, Kind::KeyRemapPointer];

fn seat(uid: Option<Uid>) -> ActiveUid {
    ActiveUid(uid)
}

#[test]
fn the_genuine_programs_get_their_own_devices_and_only_those() {
    for kind in PALMREST_KINDS {
        assert_eq!(
            authorise(kind, &palmrest(), seat(Some(USER))),
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
            authorise(kind, &palmrest(), seat(Some(USER))).is_err(),
            "palmrest asking for {kind:?}"
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
        authorise(Kind::PalmrestWheel, &palmrest(), seat(None)),
        Ok(())
    );
}

#[test]
fn refusals_table() {
    use Refusal::*;
    let user_palmrest =
        "/user.slice/user-1000.slice/user@1000.service/app.slice/bardo-mouse-gestures.service";
    let user_key_remap =
        "/user.slice/user-1000.slice/user@1000.service/app.slice/bardo-key-remap.service";
    let table: Vec<(&str, Kind, Caller, ActiveUid, Refusal)> = vec![
        (
            "not in the table",
            Kind::PalmrestWheel,
            Caller {
                credential: None,
                ..palmrest()
            },
            seat(Some(USER)),
            Unlisted,
        ),
        (
            "another role: the shell",
            Kind::PalmrestWheel,
            listed("shell", Proof::Full, PALMREST_UID, PALMREST_CGROUP),
            seat(Some(USER)),
            WrongRole {
                have: "shell".to_owned(),
            },
        ),
        (
            "palmrest asks for key_remap's keyboard",
            Kind::KeyRemapKeyboard,
            palmrest(),
            seat(Some(USER)),
            WrongRole {
                have: "pad_daemon".to_owned(),
            },
        ),
        (
            "key_remap asks for palmrest's keys",
            Kind::PalmrestKeys,
            key_remap(),
            seat(Some(USER)),
            WrongRole {
                have: "keymap_daemon".to_owned(),
            },
        ),
        (
            "palmrest proven by its control group only",
            Kind::PalmrestTouchpad,
            listed("pad_daemon", Proof::Cgroup, PALMREST_UID, PALMREST_CGROUP),
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
            "palmrest running as the logged in user",
            Kind::PalmrestWheel,
            listed("pad_daemon", Proof::Full, USER, PALMREST_CGROUP),
            seat(Some(USER)),
            WrongUser,
        ),
        (
            "palmrest as a user unit of the person",
            Kind::PalmrestWheel,
            listed("pad_daemon", Proof::Full, PALMREST_UID, user_palmrest),
            seat(Some(USER)),
            WrongPlace,
        ),
        (
            "palmrest in a lookalike system unit",
            Kind::PalmrestWheel,
            listed(
                "pad_daemon",
                Proof::Full,
                PALMREST_UID,
                "/system.slice/bardo-mouse-gestures.service/x",
            ),
            seat(Some(USER)),
            WrongPlace,
        ),
        (
            "palmrest in key_remap's unit",
            Kind::PalmrestWheel,
            listed(
                "pad_daemon",
                Proof::Full,
                PALMREST_UID,
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
        let pad = listed("pad_daemon", *proof, PALMREST_UID, PALMREST_CGROUP);
        let key = listed(
            "keymap_daemon",
            *proof,
            KEY_REMAP_UID,
            "/system.slice/bardo-key-remap.service",
        );
        for kind in PALMREST_KINDS {
            assert_eq!(
                authorise(kind, &pad, seat(Some(USER))),
                *want,
                "palmrest, {name}, {kind:?}"
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
