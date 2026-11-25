//! The error names and the bus names are the documented ones.

use memory_core::Refusal;
use memory_dbus::*;
use zbus::DBusError;

#[test]
fn refusals_map_one_to_one_to_error_names() {
    let refusals = [
        (Refusal::NotAllowed, "NotAllowed"),
        (Refusal::SpaceLocked, "SpaceLocked"),
        (Refusal::SpaceUnknown, "SpaceUnknown"),
        (Refusal::OutsideSpace, "OutsideSpace"),
        (Refusal::PlanStale, "PlanStale"),
        (Refusal::PlanExpired, "PlanExpired"),
        (Refusal::NoSuchFact, "NoSuchFact"),
        (Refusal::NotPending, "NotPending"),
        (Refusal::Busy, "Busy"),
        (Refusal::SpaceFull, "SpaceFull"),
        (Refusal::Unavailable, "Unavailable"),
        (Refusal::NotKept(memory_core::DropReason::Paused), "NotKept"),
        (Refusal::Invalid("x".into()), "Invalid"),
    ];
    let mut names = std::collections::BTreeSet::new();
    for (refusal, variant) in refusals {
        let error = MemoryError::from(&refusal);
        let name = error.name().to_string();
        assert_eq!(name, format!("{ERROR_PREFIX}.{variant}"));
        assert_eq!(error.refusal(), Some(refusal));
        names.insert(name);
    }
    assert_eq!(names.len(), 13);
}

#[test]
fn every_refusal_says_something_in_words() {
    for refusal in [
        Refusal::NotAllowed,
        Refusal::SpaceLocked,
        Refusal::SpaceUnknown,
        Refusal::OutsideSpace,
        Refusal::PlanStale,
        Refusal::PlanExpired,
        Refusal::NoSuchFact,
        Refusal::NotPending,
        Refusal::Busy,
        Refusal::SpaceFull,
        Refusal::Unavailable,
    ] {
        let error = MemoryError::from(&refusal);
        let words = error.description().unwrap_or_default();
        assert!(!words.trim().is_empty(), "{refusal:?} has no message");
    }
}
