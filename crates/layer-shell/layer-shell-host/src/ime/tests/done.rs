//! `ime::on_done`: preedit/commit/delete sequences accumulated over several wire events, applied
//! once `done` arrives, against the public API only. The double-buffering itself (accumulate
//! into `PendingIme`, apply and reset on `done`) is what `handlers/text_input.rs`'s `Dispatch`
//! impl drives; these tests build the same `PendingIme` a sequence of events would leave behind
//! and check what `on_done` does with it.

use crate::ime::{DeleteSurrounding, Preedit, on_done};
use blitz_traits::events::BlitzImeEvent;

/// One step in a wire sequence: which buffered field it sets (`PendingIme` is double-buffered,
/// so later events of the same kind in one cycle replace earlier ones; every `done` resets the
/// shown preedit, so anything but a lone new preedit starts with a clear).
#[derive(Debug, Clone, Default)]
struct Pending {
    preedit: Option<Preedit>,
    commit: Option<&'static str>,
    delete: Option<(u32, u32)>,
}

/// The preedit reset every `done` starts with unless a new preedit replaces it.
fn clear() -> BlitzImeEvent {
    BlitzImeEvent::Preedit(String::new(), None)
}

fn pending(steps: Pending) -> crate::ime::PendingIme {
    crate::ime::PendingIme {
        preedit: steps.preedit,
        commit: steps.commit.map(str::to_owned),
        delete: steps
            .delete
            .map(|(before, after)| DeleteSurrounding { before, after }),
    }
}

fn preedit(text: &str, cursor: Option<(usize, usize)>) -> Option<Preedit> {
    Some(Preedit {
        text: text.to_owned(),
        cursor,
    })
}

fn deleted(before: usize, after: usize) -> BlitzImeEvent {
    BlitzImeEvent::DeleteSurrounding {
        before_bytes: before,
        after_bytes: after,
    }
}

/// Each row is a buffered sequence and the events its `done` applies; every `done` leaves the
/// buffer empty. The protocol's fixed order is clear the old preedit, delete, commit, new preedit,
/// whatever order the wire events arrived in; a lone new preedit replaces the clear.
#[test]
fn a_done_applies_what_was_buffered_in_the_protocols_order() {
    let cases: Vec<(&str, Pending, Vec<BlitzImeEvent>)> = vec![
        (
            "nothing pending only clears the preedit",
            Pending::default(),
            vec![clear()],
        ),
        (
            "a single preedit commits its cursor range",
            Pending {
                preedit: preedit("ni", Some((0, 2))),
                ..Pending::default()
            },
            vec![BlitzImeEvent::Preedit("ni".to_owned(), Some((0, 2)))],
        ),
        (
            "both cursors at -1 hide the cursor",
            Pending {
                preedit: preedit("n", None),
                ..Pending::default()
            },
            vec![BlitzImeEvent::Preedit("n".to_owned(), None)],
        ),
        (
            "a single commit is one event",
            Pending {
                commit: Some("你"),
                ..Pending::default()
            },
            vec![clear(), BlitzImeEvent::Commit("你".to_owned())],
        ),
        (
            "an empty commit string is still a commit event",
            Pending {
                commit: Some(""),
                ..Pending::default()
            },
            vec![clear(), BlitzImeEvent::Commit(String::new())],
        ),
        (
            "a single delete is one event",
            Pending {
                delete: Some((1, 0)),
                ..Pending::default()
            },
            vec![clear(), deleted(1, 0)],
        ),
        (
            "a full cycle applies delete, then commit, then preedit",
            Pending {
                preedit: preedit("ni", Some((0, 2))),
                commit: Some("你"),
                delete: Some((1, 0)),
            },
            vec![
                clear(),
                deleted(1, 0),
                BlitzImeEvent::Commit("你".to_owned()),
                BlitzImeEvent::Preedit("ni".to_owned(), Some((0, 2))),
            ],
        ),
        // Bopomofo's `s u 3 c l 3` then Return, as fcitx5 sends it: the preedit 你好 is replaced
        // by nothing and 你好 is committed in one `done`. Committing with the composition still
        // shown doubled it (你好你好).
        (
            "a commit of the composed text clears the composition first",
            Pending {
                preedit: preedit("", Some((0, 0))),
                commit: Some("你好"),
                ..Pending::default()
            },
            vec![
                clear(),
                BlitzImeEvent::Commit("你好".to_owned()),
                BlitzImeEvent::Preedit(String::new(), Some((0, 0))),
            ],
        ),
    ];
    for (name, buffered, want) in cases {
        let (events, rest) = on_done(pending(buffered));
        assert_eq!(events, want, "{name}");
        assert_eq!(rest, crate::ime::PendingIme::default(), "{name}");
    }
}

/// Two `done` cycles in a row: the second sees only what was buffered since the first (the
/// double-buffering resets, it does not accumulate across cycles).
#[test]
fn a_second_done_only_sees_what_was_buffered_since_the_first() {
    let (first_events, after_first) = on_done(pending(Pending {
        commit: Some("a"),
        ..Pending::default()
    }));
    assert_eq!(
        first_events,
        vec![clear(), BlitzImeEvent::Commit("a".to_owned())]
    );

    // Nothing new buffered before the next `done`: only a clear, not a repeat of "a".
    let (second_events, _) = on_done(after_first);
    assert_eq!(second_events, vec![clear()]);
}
