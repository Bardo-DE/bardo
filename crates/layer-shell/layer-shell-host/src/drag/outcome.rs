//! How a drag ended, from the source's last events.

use layer_shell_types::{DragAction, DragOutcome};
use wayland_client::protocol::wl_data_device_manager::DndAction;

/// The action a `wl_data_source.action` event chose; `None` for none, or for `ask` (the target
/// settles an ask with a later `action` before it finishes).
#[must_use]
pub(crate) fn action_from_wire(action: DndAction) -> Option<DragAction> {
    if action == DndAction::Copy {
        Some(DragAction::Copy)
    } else if action == DndAction::Move {
        Some(DragAction::Move)
    } else {
        None
    }
}

/// The outcome of a drag whose source's last event was `dnd_finished` (`finished`: the target
/// read what it wanted and is done) or `cancelled` (no target took it, or the target gave up
/// after the drop), the last action the compositor chose being `chosen`. A finish with no
/// action chosen is a copy: the source must not delete anything it was not told to move.
pub(crate) fn outcome(finished: bool, chosen: Option<DragAction>) -> DragOutcome {
    if finished {
        DragOutcome::Dropped {
            action: chosen.unwrap_or(DragAction::Copy),
        }
    } else {
        DragOutcome::Cancelled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_end_and_the_chosen_action_make_the_outcome() {
        let dropped = |action| DragOutcome::Dropped { action };
        let cases = [
            (true, Some(DragAction::Copy), dropped(DragAction::Copy)),
            (true, Some(DragAction::Move), dropped(DragAction::Move)),
            (true, None, dropped(DragAction::Copy)),
            (false, Some(DragAction::Move), DragOutcome::Cancelled),
            (false, None, DragOutcome::Cancelled),
        ];
        for (end, chosen, want) in cases {
            assert_eq!(outcome(end, chosen), want, "finished: {end}, {chosen:?}");
        }
    }

    #[test]
    fn only_a_single_copy_or_move_is_an_action() {
        let cases = [
            (DndAction::Copy, Some(DragAction::Copy)),
            (DndAction::Move, Some(DragAction::Move)),
            (DndAction::Ask, None),
            (DndAction::empty(), None),
            (DndAction::Copy | DndAction::Move, None),
        ];
        for (wire, want) in cases {
            assert_eq!(action_from_wire(wire), want, "{wire:?}");
        }
    }
}
