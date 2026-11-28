use crate::support::{KEY_REMAP_UID, MOUSE_GESTURES_UID, USER, key_remap, mouse_gestures};
use caller_identity_core::Uid;
use input_broker_core::identity::{ActiveUid, Caller};
use input_broker_core::limits::{
    MAX_CONNECTIONS, MAX_CONNECTIONS_PER_USER, MAX_RESERVED_PER_CLIENT,
};
use input_broker_core::slots::{Bucket, NoSlotError, Slots};
use input_broker_wire::Maker;

fn seat() -> ActiveUid {
    ActiveUid(Some(USER))
}

fn stranger(uid: Uid) -> Caller {
    Caller {
        uid,
        credential: None,
        ..mouse_gestures()
    }
}

#[test]
fn a_caller_that_passes_the_policy_has_a_reserve_and_anyone_else_a_user_pool() {
    let table: &[(&str, Caller, Bucket)] = &[
        (
            "the mouse gestures daemon",
            mouse_gestures(),
            Bucket::Reserved(Maker::PadDaemon),
        ),
        (
            "the key remap daemon",
            key_remap(),
            Bucket::Reserved(Maker::KeymapDaemon),
        ),
        (
            "a stranger at the seat",
            stranger(USER),
            Bucket::Stranger(USER),
        ),
        (
            "a stranger elsewhere",
            stranger(Uid::new(2000)),
            Bucket::Stranger(Uid::new(2000)),
        ),
        (
            "mouse_gestures's user is not a free pass",
            Caller {
                credential: None,
                ..mouse_gestures()
            },
            Bucket::Stranger(MOUSE_GESTURES_UID),
        ),
        (
            "mouse_gestures running as the person at the seat",
            Caller {
                uid: USER,
                ..mouse_gestures()
            },
            Bucket::Stranger(USER),
        ),
        (
            "key_remap's user is not a free pass",
            Caller {
                credential: None,
                ..key_remap()
            },
            Bucket::Stranger(KEY_REMAP_UID),
        ),
    ];
    for (name, caller, want) in table {
        assert_eq!(Bucket::of(caller, seat()), *want, "{name}");
    }
}

#[test]
fn one_user_cannot_take_another_users_slots_or_the_reserve() {
    let slots = Slots::new();
    let mine: Vec<_> = (0..MAX_CONNECTIONS_PER_USER)
        .map(|_| slots.take(Bucket::Stranger(Uid::new(1))).expect("a slot"))
        .collect();
    assert_eq!(
        slots.take(Bucket::Stranger(Uid::new(1))).err(),
        Some(NoSlotError)
    );
    let theirs = slots
        .take(Bucket::Stranger(Uid::new(2)))
        .expect("another user's slot");
    let pad = slots
        .take(Bucket::Reserved(Maker::PadDaemon))
        .expect("reserve");
    assert_eq!(slots.open(), MAX_CONNECTIONS_PER_USER + 2);
    drop(mine);
    assert_eq!(slots.open(), 2);
    assert!(
        slots.take(Bucket::Stranger(Uid::new(1))).is_ok(),
        "a dropped slot is free again"
    );
    drop((theirs, pad));
}

#[test]
fn many_users_share_the_stranger_total_but_never_the_reserve() {
    let slots = Slots::new();
    let held: Vec<_> = (0..MAX_CONNECTIONS)
        .map(|uid| {
            slots
                .take(Bucket::Stranger(Uid::new(
                    u32::try_from(uid).expect("small"),
                )))
                .expect("a slot")
        })
        .collect();
    assert_eq!(
        slots.take(Bucket::Stranger(Uid::new(9999))).err(),
        Some(NoSlotError)
    );
    for client in [Maker::PadDaemon, Maker::KeymapDaemon] {
        let reserve: Vec<_> = (0..MAX_RESERVED_PER_CLIENT)
            .map(|_| slots.take(Bucket::Reserved(client)).expect("reserve"))
            .collect();
        assert_eq!(
            slots.take(Bucket::Reserved(client)).err(),
            Some(NoSlotError)
        );
        drop(reserve);
    }
    drop(held);
    assert_eq!(slots.open(), 0);
}
