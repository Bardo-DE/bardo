//! Where the pointer last moved in this root. The root hears every move last (it bubbles there
//! from every surface, the overlays' menus included), so a handler that asks before it is the
//! one that knows whether this move went anywhere. A move to where the pointer already was is
//! the hover repair replaying the resting pointer after the layout changed under it
//! (`blitz_kit::hover`): a browser sends no move then, and a list that follows the pointer must
//! not follow it, or a menu closing over a list selects the row under the still pointer.

use crate::host::event_point::pointer_point;
use dioxus::prelude::*;
use ds_core::geometry::units::Point;

/// The last point a move reached in this root.
#[derive(Clone, Copy)]
pub(crate) struct PointerTrail(CopyValue<Option<Point>>);

impl PointerTrail {
    /// The root's trail, for its children.
    pub(crate) fn provide() -> Self {
        use_context_provider(|| Self(CopyValue::new(None)))
    }

    /// The root heard `event`, after every handler under it.
    pub(crate) fn heard(self, event: &PointerEvent) {
        let mut last = self.0;
        last.set(Some(pointer_point(event)));
    }

    /// Whether a move to `to`, heard before the root, went anywhere: the pointer is somewhere it
    /// was not at the last move. `false` when the pointer is where the last move left it, so the
    /// move was replayed, not made.
    pub(crate) fn moved(self, to: Point) -> bool {
        *self.0.peek() != Some(to)
    }
}

/// The trail of the root this component is in; outside a root every move counts as made.
pub(crate) fn use_pointer_trail() -> Option<PointerTrail> {
    use_hook(try_consume_context::<PointerTrail>)
}

/// Whether a move to `to` went anywhere, by `trail` when there is one.
pub(crate) fn moved_to(trail: Option<PointerTrail>, to: Point) -> bool {
    trail.is_none_or(|trail| trail.moved(to))
}
