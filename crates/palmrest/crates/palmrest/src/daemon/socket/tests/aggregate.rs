//! How many subscribers' words become one.

use super::super::aggregate::{aggregate, aggregate_lock};

#[test]
fn the_session_is_locked_unless_a_subscriber_has_said_unlocked_and_none_says_locked() {
    const LOCKED: bool = true;
    const UNLOCKED: bool = false;
    let table: &[(&str, Vec<Option<bool>>, bool)] = &[
        ("no subscriber is connected", vec![], LOCKED),
        ("one that has said nothing", vec![None], LOCKED),
        ("several that have said nothing", vec![None, None], LOCKED),
        ("one that says unlocked", vec![Some(UNLOCKED)], UNLOCKED),
        (
            "unlocked beside one that is silent",
            vec![None, Some(UNLOCKED)],
            UNLOCKED,
        ),
        ("one that says locked", vec![Some(LOCKED)], LOCKED),
        (
            "locked beats unlocked",
            vec![Some(UNLOCKED), Some(LOCKED)],
            LOCKED,
        ),
        (
            "locked beats unlocked, either order",
            vec![Some(LOCKED), Some(UNLOCKED), None],
            LOCKED,
        ),
    ];
    for (name, said, want) in table {
        assert_eq!(aggregate_lock(said.iter().copied()), *want, "{name}");
    }
}

#[test]
fn any_claim_wins() {
    assert!(!aggregate([]));
    assert!(aggregate([false, true]));
    assert!(!aggregate([false, false]));
}
