//! Scroll notification to components (design/11 §11.3.15): each frame the host publishes the
//! offset, extent and phase of every scroller a component watches with [`use_scroll`]. Blitz's
//! own `onscroll` never fires for the engine's raw writes, so this is the one way to follow a
//! scroll.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::core::{spawn, use_hook};
use dioxus::hooks::use_signal;
use dioxus::signals::{ReadSignal, WritableExt};
use layer_shell_types::{ElementId, ScrollState};

use crate::handle::use_surface;
use crate::live::Live;

/// The scrollers a surface's components watch, by element id.
///
/// Shared between the host (which
/// publishes) and the surface's `SurfaceHandle`, on the main thread. An entry stays for the
/// surface's life once watched.
#[derive(Debug, Clone, Default)]
pub struct ScrollBoard(Rc<RefCell<HashMap<ElementId, Live<ScrollState>>>>);

impl ScrollBoard {
    /// The live state of `id`, created on first watch.
    #[must_use]
    pub fn watch(&self, id: &ElementId) -> Live<ScrollState> {
        self.0
            .borrow_mut()
            .entry(id.clone())
            .or_insert_with(|| Live::new(ScrollState::default()))
            .clone()
    }

    /// Every watched id and its live state.
    #[must_use]
    pub fn watched(&self) -> Vec<(ElementId, Live<ScrollState>)> {
        self.0
            .borrow()
            .iter()
            .map(|(id, live)| (id.clone(), live.clone()))
            .collect()
    }
}

/// The scroll state of the scroll container whose `id` attribute is `id` in this component's
/// surface.
///
/// It is live state: re-rendered when its offset, extent or phase changes (at most once per frame).
/// `ScrollState::default()` until the element has laid out.
#[must_use]
pub fn use_scroll(id: &ElementId) -> ReadSignal<ScrollState> {
    let board = use_surface().scroll_board();
    let live = use_hook(|| board.watch(id));
    let mut latest = use_signal(|| live.get());
    use_hook(move || {
        let mut seen = live.changes();
        spawn(async move {
            loop {
                let (count, state) = live.changed_after(seen).await;
                seen = count;
                latest.set(state);
            }
        });
    });
    ReadSignal::from(latest)
}
