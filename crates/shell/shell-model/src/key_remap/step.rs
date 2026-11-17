//! The pure transition and the decision to tell the person.

use key_remap_proto::{Grab, Status};

use super::command::{KeyRemapEvent, UnitState};
use super::model::{Attention, KeyRemapState};

/// `state` after `event`.
///
/// The socket decides `Connected` and a drop out of it; the unit decides everything else, so a drop
/// followed by `Failed` is `Failed` and one followed by `Active` and a new `Hello` is back to
/// `Connected` without anyone being told.
#[must_use]
#[expect(
    clippy::match_same_arms,
    reason = "the arms are the machine's table, in the order they are tried: equal results of different events are different cases, and an earlier arm sits between the ones that coincide"
)]
pub fn step(state: KeyRemapState, event: KeyRemapEvent) -> KeyRemapState {
    match (state, event) {
        (_, KeyRemapEvent::Hello(status) | KeyRemapEvent::Status(status)) => {
            KeyRemapState::Connected { status }
        }
        (KeyRemapState::Connected { .. }, KeyRemapEvent::Disconnected) => KeyRemapState::Restarting,
        (state, KeyRemapEvent::Disconnected) => state,
        (_, KeyRemapEvent::UnitState(UnitState::Failed)) => KeyRemapState::Failed,
        (_, KeyRemapEvent::UnitState(UnitState::NotFound)) => KeyRemapState::NotInstalled,
        (_, KeyRemapEvent::UnitState(UnitState::Inactive)) => KeyRemapState::Unknown,
        (_, KeyRemapEvent::UnitState(UnitState::Activating)) => KeyRemapState::Restarting,
        (
            state @ (KeyRemapState::Connected { .. } | KeyRemapState::Unknown),
            KeyRemapEvent::UnitState(UnitState::Active),
        ) => state,
        (_, KeyRemapEvent::UnitState(UnitState::Active)) => KeyRemapState::Restarting,
    }
}

/// What the person should be told, if anything. A conflict outranks a pause: it needs them to
/// act on another program, a pause only a key chord.
#[must_use]
pub fn attention(state: &KeyRemapState) -> Option<Attention> {
    match state {
        KeyRemapState::Failed => Some(Attention::Failed),
        KeyRemapState::Connected { status } => conflict(status).or_else(|| paused(status)),
        KeyRemapState::Unknown | KeyRemapState::Restarting | KeyRemapState::NotInstalled => None,
    }
}

fn conflict(status: &Status) -> Option<Attention> {
    status
        .keyboards
        .iter()
        .find(|keyboard| keyboard.grab == Grab::Conflict)
        .map(|keyboard| Attention::Conflict {
            device: keyboard.info.name.clone(),
        })
}

fn paused(status: &Status) -> Option<Attention> {
    status.paused.then_some(Attention::Paused)
}
