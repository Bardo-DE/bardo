//! The phase the scroll engine gives a watched scroller. The watch itself (`ScrollBoard`,
//! `use_scroll`) is `layer_shell_hooks`'s; the host publishes into it each frame.

use blitz_embed::scroll::engine::Engine;
use blitz_embed::scroll::geom::Scroller;
use layer_shell_types::ScrollPhase;

/// The phase `engine` gives `scroller`.
pub(crate) fn phase_of(engine: &Engine, scroller: Scroller) -> ScrollPhase {
    if engine.latch().map(|latch| latch.scroller) != Some(scroller) {
        return ScrollPhase::Idle;
    }
    match engine {
        Engine::Idle | Engine::Pending(_) => ScrollPhase::Idle,
        Engine::Tracking(_) => ScrollPhase::Tracking,
        Engine::Momentum(_) => ScrollPhase::Momentum,
        Engine::Rebound(_) => ScrollPhase::Rebound,
        Engine::Smooth(_) | Engine::Held(_) | Engine::Spring(_) => ScrollPhase::Animating,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::task::Context;

    use blitz_traits::shell::{ColorScheme, Viewport};
    use dioxus::prelude::*;

    use super::*;
    use crate::clipboard::SharedSelection;
    use crate::dom::document::{DocumentEnv, SurfaceDocument};
    use crate::dom::shell_provider::HostShellProvider;
    use crate::prepaint::Prepaint;
    use crate::scroll::surface::SurfaceScroll;
    use crate::scroll::tuning::Tuning;
    use blitz_embed::fonts::SharedFonts;
    use blitz_embed::net::LocalNet;
    use blitz_embed::scroll::doc as dom;
    use blitz_embed::scroll::geom::ScrollAxis;
    use layer_shell_hooks::{
        FrameTicks, GrabRecord, HandleParts, HostRoot, Live, NativeCell, RequestQueue, RoleKind,
        ScrollBoard, SurfaceHandle, SurfaceIds, WheelFeed, use_scroll,
    };
    use layer_shell_types::{
        ElementId, InputModality, Modifiers, PlacementSupport, Scale120, SurfaceId, ToplevelState,
    };

    /// Shows a scroll box's offset as text, through `use_scroll`.
    fn watcher() -> Element {
        let state = use_scroll(&ElementId::new("box"));
        let y = state.read().offset.y;
        rsx! {
            div { id: "box", style: "height:100px;overflow:auto",
                div { style: "height:500px" }
            }
            p { id: "out", "{y}" }
        }
    }

    fn out(doc: &SurfaceDocument) -> String {
        let base = doc.base();
        let id = base.get_element_by_id("out").expect("out");
        base.get_node(id).expect("node").text_content()
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "exact by construction (clamped, copied or integer-valued), so exact equality is the assertion and a tolerance would hide drift"
    )]
    fn use_scroll_follows_what_the_host_publishes() {
        let (ping, _source) = calloop::ping::make_ping().expect("eventfd ping");
        let board = ScrollBoard::default();
        let parts = HandleParts {
            requests: RequestQueue::new({
                let ping = ping.clone();
                move || ping.ping()
            }),
            ids: SurfaceIds::new(),
            ticks: FrameTicks::default(),
            scale: Live::new(Scale120::ONE),
            toplevel: Live::new(ToplevelState::default()),
            grab: GrabRecord::default(),
            scrolls: board.clone(),
            wheels: WheelFeed::default(),
            modifiers: Live::new(Modifiers::default()),
            modality: Live::new(InputModality::default()),
            native: NativeCell::detached(),
            placement: PlacementSupport::Absent,
        };
        let handle = SurfaceHandle::new(SurfaceId(3), RoleKind::Toplevel, parts);
        let env = DocumentEnv {
            shell: Arc::new(HostShellProvider::new(ping, SharedSelection::default())),
            fonts: SharedFonts::system().for_document(),
            net: Arc::new(LocalNet::default()),
            viewport: Viewport::new(200, 300, 1.0, ColorScheme::Light),
            prepaint: Prepaint::None,
        };
        let mut doc = SurfaceDocument::build(HostRoot::new(watcher).context(handle), env);
        let mut cx = Context::from_waker(std::task::Waker::noop());
        doc.poll(&mut cx);
        doc.resolve(0.0);
        assert_eq!(out(&doc), "0");
        let scroll = SurfaceScroll::new(board, Tuning::default());
        let target = dom::scroller_by_id(&doc.base(), &ElementId::new("box")).expect("box");
        dom::write(&mut doc.base_mut(), target, ScrollAxis::Y, 123.0);
        scroll.publish(&doc.base());
        doc.poll(&mut cx);
        doc.resolve(0.1);
        assert_eq!(out(&doc), "123");
        let published = scroll.board().watch(&ElementId::new("box")).get();
        assert_eq!(published.extent.content_height, 500.0);
        assert_eq!(published.extent.viewport_height, 100.0);
        assert_eq!(published.phase, layer_shell_types::ScrollPhase::Idle);
    }
}
