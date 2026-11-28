use super::*;

#[test]
fn role_names_round_trip() {
    const CASES: &[(&str, Option<Role>)] = &[
        ("command", Some(Role::Command)),
        ("option", Some(Role::Option)),
        ("control", Some(Role::Control)),
        ("shift", Some(Role::Shift)),
        ("ctrl", None),
        ("Command", None),
        ("", None),
    ];
    for &(name, want) in CASES {
        assert_eq!(Role::named(name), want, "{name:?}");
        if let Some(role) = want {
            assert_eq!(role.name(), name);
        }
    }
}

#[test]
fn roles_set_with_without_contains_order_and_bits() {
    // Step 1: with, without and contains.
    let held = Roles::EMPTY.with(Role::Command).with(Role::Shift);
    assert!(Roles::EMPTY.is_empty());
    assert!(!held.is_empty());
    assert!(held.contains(Role::Command));
    assert!(held.contains(Role::Shift));
    assert!(!held.contains(Role::Option));
    let released = held.without(Role::Command);
    assert!(!released.contains(Role::Command));
    assert!(released.contains(Role::Shift));
    assert_eq!(released.without(Role::Shift), Roles::EMPTY);
    assert_eq!(held.with(Role::Command), held, "with is idempotent");
    assert_eq!(Roles::EMPTY.without(Role::Control), Roles::EMPTY);

    // Step 2: iteration is in declaration order.
    let held: Roles = [Role::Shift, Role::Command, Role::Control]
        .into_iter()
        .collect();
    assert_eq!(
        held.iter().collect::<Vec<_>>(),
        [Role::Command, Role::Control, Role::Shift]
    );
    assert_eq!(Roles::EMPTY.iter().count(), 0);

    // Step 3: every member has its own bit.
    for (i, a) in Role::ALL.into_iter().enumerate() {
        for b in Role::ALL[i + 1..].iter().copied() {
            assert!(!Roles::EMPTY.with(a).contains(b), "{a:?} {b:?}");
        }
    }
    let all: Roles = Role::ALL.into_iter().collect();
    assert_eq!(all.iter().count(), Role::ALL.len());
}
