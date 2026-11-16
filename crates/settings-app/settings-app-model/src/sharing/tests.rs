//! The sharing switch's transitions.

use super::{Existing, LendFailure, Sharing, SharingEffect, SharingInput, sharing_step};

fn failed() -> Result<(), LendFailure> {
    Err(LendFailure::CouldNotRestart)
}

type Case = (Sharing, SharingInput, Sharing, Vec<SharingEffect>);

#[test]
fn the_transitions() {
    let cases: Vec<Case> = vec![
        (
            Sharing::Idle,
            SharingInput::Switched(true, Existing::Absent),
            Sharing::Asking(Existing::Absent),
            vec![],
        ),
        (
            Sharing::Idle,
            SharingInput::Switched(false, Existing::Same),
            Sharing::Working(false),
            vec![SharingEffect::Disable],
        ),
        (
            Sharing::Asking(Existing::Same),
            SharingInput::Allowed,
            Sharing::Working(true),
            vec![SharingEffect::Enable],
        ),
        (
            Sharing::Asking(Existing::Same),
            SharingInput::Cancelled,
            Sharing::Idle,
            vec![],
        ),
        (
            Sharing::Working(true),
            SharingInput::Finished(Ok(())),
            Sharing::Idle,
            vec![SharingEffect::Record(true)],
        ),
        (
            Sharing::Working(true),
            SharingInput::Finished(failed()),
            Sharing::Idle,
            vec![SharingEffect::Tell(LendFailure::CouldNotRestart)],
        ),
        (
            Sharing::Working(false),
            SharingInput::Finished(Ok(())),
            Sharing::Idle,
            vec![SharingEffect::Record(false)],
        ),
        (
            Sharing::Working(false),
            SharingInput::Finished(failed()),
            Sharing::Idle,
            vec![
                SharingEffect::Record(false),
                SharingEffect::Tell(LendFailure::CouldNotRestart),
            ],
        ),
        // Presses that mean nothing now change nothing.
        (
            Sharing::Working(true),
            SharingInput::Switched(false, Existing::Same),
            Sharing::Working(true),
            vec![],
        ),
        (Sharing::Idle, SharingInput::Allowed, Sharing::Idle, vec![]),
    ];
    for (state, input, next, effects) in cases {
        let shown = format!("{state:?} + {input:?}");
        assert_eq!(sharing_step(state, input), (next, effects), "{shown}");
    }
}
