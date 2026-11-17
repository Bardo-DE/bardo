//! The xdg-toplevel wire values each toplevel choice becomes, and a configure read back as a
//! `ToplevelState`. The choices and the state are `layer_shell_types`'s.

use layer_shell_types::{
    Activation, Decorations, Fullscreen, Maximize, ResizeEdge, Tile, TileEdges, ToplevelState,
};
use smithay_client_toolkit::shell::xdg::window::{WindowConfigure, WindowDecorations};
use wayland_protocols::xdg::shell::client::xdg_toplevel;

/// The sctk request for `decorations`.
pub(crate) fn to_wire_decorations(decorations: Decorations) -> WindowDecorations {
    match decorations {
        Decorations::Client(_) => WindowDecorations::RequestClient,
        Decorations::Server => WindowDecorations::RequestServer,
    }
}

/// The `xdg_toplevel.resize_edge` for `edge`.
pub(crate) fn to_wire_edge(edge: ResizeEdge) -> xdg_toplevel::ResizeEdge {
    match edge {
        ResizeEdge::Top => xdg_toplevel::ResizeEdge::Top,
        ResizeEdge::Bottom => xdg_toplevel::ResizeEdge::Bottom,
        ResizeEdge::Left => xdg_toplevel::ResizeEdge::Left,
        ResizeEdge::Right => xdg_toplevel::ResizeEdge::Right,
        ResizeEdge::TopLeft => xdg_toplevel::ResizeEdge::TopLeft,
        ResizeEdge::TopRight => xdg_toplevel::ResizeEdge::TopRight,
        ResizeEdge::BottomLeft => xdg_toplevel::ResizeEdge::BottomLeft,
        ResizeEdge::BottomRight => xdg_toplevel::ResizeEdge::BottomRight,
    }
}

/// The state a configure carries.
pub(crate) fn state_of(configure: &WindowConfigure) -> ToplevelState {
    ToplevelState {
        maximized: Maximize::from(configure.is_maximized()),
        fullscreen: Fullscreen::from(configure.is_fullscreen()),
        activated: Activation::from(configure.is_activated()),
        tiled: TileEdges {
            top: Tile::from(configure.is_tiled_top()),
            bottom: Tile::from(configure.is_tiled_bottom()),
            left: Tile::from(configure.is_tiled_left()),
            right: Tile::from(configure.is_tiled_right()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use layer_shell_types::ResizeMargin;

    #[test]
    fn decorations_map_to_sctk_requests() {
        let cases = [
            (Decorations::CLIENT, WindowDecorations::RequestClient),
            (
                Decorations::Client(ResizeMargin(0)),
                WindowDecorations::RequestClient,
            ),
            (Decorations::Server, WindowDecorations::RequestServer),
        ];
        for (ours, wire) in cases {
            assert_eq!(to_wire_decorations(ours), wire, "{ours:?}");
        }
    }

    #[test]
    fn every_edge_maps_to_its_xdg_resize_edge() {
        use xdg_toplevel::ResizeEdge as W;
        let want = [
            W::Top,
            W::Bottom,
            W::Left,
            W::Right,
            W::TopLeft,
            W::TopRight,
            W::BottomLeft,
            W::BottomRight,
        ];
        for (edge, wire) in ResizeEdge::ALL.into_iter().zip(want) {
            assert_eq!(to_wire_edge(edge), wire, "{edge:?}");
        }
    }
}
