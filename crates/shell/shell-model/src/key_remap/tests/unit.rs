use key_remap_proto::Status;

use crate::key_remap::{KeyRemapState, UnitState, unit_state, watching_unit};

#[test]
fn the_managers_words_become_unit_states() {
    let cases: &[(&str, Option<UnitState>)] = &[
        ("active", Some(UnitState::Active)),
        ("reloading", Some(UnitState::Active)),
        ("activating", Some(UnitState::Activating)),
        ("deactivating", Some(UnitState::Activating)),
        ("failed", Some(UnitState::Failed)),
        ("maintenance", Some(UnitState::Failed)),
        ("inactive", Some(UnitState::Inactive)),
        ("", None),
        ("Active", None),
        ("something-new", None),
    ];
    for &(word, want) in cases {
        assert_eq!(unit_state(word), want, "{word:?}");
    }
}

#[test]
fn the_unit_is_asked_about_only_while_the_socket_is_down() {
    let connected = KeyRemapState::Connected {
        status: Status {
            paused: false,
            keyboards: Vec::new(),
        },
    };
    let cases = [
        ("unknown", KeyRemapState::Unknown, true),
        ("restarting", KeyRemapState::Restarting, true),
        ("failed", KeyRemapState::Failed, true),
        ("connected", connected, false),
        ("not installed", KeyRemapState::NotInstalled, false),
    ];
    for (case, state, want) in cases {
        assert_eq!(watching_unit(&state), want, "{case}");
    }
}
