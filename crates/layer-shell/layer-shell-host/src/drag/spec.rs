//! The `wl_data_source` action set a `DragActions` becomes.

use layer_shell_types::DragActions;
use wayland_client::protocol::wl_data_device_manager::DndAction;

/// The wire action set.
#[must_use]
pub(crate) fn actions_to_wire(actions: DragActions) -> DndAction {
    match actions {
        DragActions::Copy => DndAction::Copy,
        DragActions::Move => DndAction::Move,
        DragActions::CopyOrMove => DndAction::Copy | DndAction::Move,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_become_the_wire_set() {
        let cases = [
            (DragActions::Copy, DndAction::Copy),
            (DragActions::Move, DndAction::Move),
            (DragActions::CopyOrMove, DndAction::Copy | DndAction::Move),
        ];
        for (actions, wire) in cases {
            assert_eq!(actions_to_wire(actions), wire, "{actions:?}");
        }
    }
}
