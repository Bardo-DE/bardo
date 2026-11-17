//! The repair itself, applied to a document.

use blitz_dom::{BaseDocument, Document, NodeId};
use blitz_traits::events::{BlitzPointerEvent, UiEvent};

use super::{HoverSync, LastMove, Shift, decide};
use crate::hit::element_at;
use blitz_embed_types::units::Bounds;

/// Whether [`repair`] changed anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repaired {
    Yes,
    No,
}

/// Undoes a hover move that Blitz's `refresh_hover` made without dispatching.
///
/// It puts the old hover back without dispatch, then feeds the last move again through `doc`'s
/// own driver, so it dispatches the leave/enter diff itself. Call at most once per resolve, with the hovered elements from
/// before and after it. `doc` is any [`Document`] (a Dioxus document dispatches into its
/// components; a bare `BaseDocument` into nothing).
pub fn repair(doc: &mut dyn Document, last: &LastMove, shift: Shift) -> Repaired {
    let sync = {
        let inner = doc.inner();
        decide(
            shift,
            last,
            |id| page_bounds(&inner, id),
            |at| element_at(&inner, at),
        )
    };
    match sync {
        HoverSync::Unchanged => return Repaired::No,
        HoverSync::Restore(at) => {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "page coordinates are far inside f32's range, and f32 is Blitz's own coordinate type"
            )]
            doc.inner_mut().set_hover_to(at.x as f32, at.y as f32);
        }
        HoverSync::Clear => {
            doc.inner_mut().clear_hover();
        }
    }
    if let LastMove::At(event) = last {
        let scroll = doc.inner().viewport_scroll();
        doc.handle_ui_event(UiEvent::PointerMove(at_page(event, (scroll.x, scroll.y))));
    }
    Repaired::Yes
}

/// `event` with its page coordinates its client ones moved by the viewport's `scroll`: Blitz
/// hit-tests page coordinates, and a surface's events are surface-local.
fn at_page(event: &BlitzPointerEvent, scroll: (f64, f64)) -> BlitzPointerEvent {
    let mut moved = event.clone();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a viewport scroll offset is far inside f32's range, and f32 is Blitz's coordinate type"
    )]
    let (scroll_x, scroll_y) = (scroll.0 as f32, scroll.1 as f32);
    moved.coords.page_x = moved.coords.client_x + scroll_x;
    moved.coords.page_y = moved.coords.client_y + scroll_y;
    moved
}

/// `id`'s current border box, in the page coordinates Blitz hit-tests (client plus the
/// viewport's scroll).
fn page_bounds(doc: &BaseDocument, id: NodeId) -> Option<Bounds> {
    let rect = doc.get_client_bounding_rect(id)?;
    let scroll = doc.viewport_scroll();
    Some(Bounds {
        x: rect.x + scroll.x,
        y: rect.y + scroll.y,
        width: rect.width,
        height: rect.height,
    })
}
