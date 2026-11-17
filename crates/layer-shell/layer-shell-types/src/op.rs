//! What a surface can be told to change: the ops a `SurfaceHandle` or a `HostHandle` applies.

use crate::blur::BlurSpec;
use crate::geometry::Margin;
use crate::layer::{Anchor, ExclusiveZone, KeyboardMode, LayerLevel, LayerSize};
use crate::placement::PopupPlacement;
use crate::region::RegionSpec;
use crate::toplevel::{Fullscreen, Maximize, ResizeEdge};
use crate::window_placement::WindowPlacement;

/// A change to one surface, grouped by the roles that can take it.
///
/// Applied after the documents are polled and before painting, so a region change lands in the same
/// commit as the frame that caused it. An op that does not fit the surface's role (a layer op on a
/// popup) is ignored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceOp {
    /// Any surface.
    Common(CommonOp),
    /// Layer surfaces only.
    Layer(LayerOp),
    /// Toplevels only.
    Window(WindowOp),
    /// Popups only: move an open popup (`xdg_popup.reposition`, `xdg_wm_base` v3), which is how a
    /// menu bar switches menus on hover without a new grab (design/13 §13.7).
    Reposition(PopupPlacement),
}

/// A change any surface takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommonOp {
    SetBlur(BlurSpec),
    SetInputRegion(RegionSpec),
    SetVisibility(Visibility),
}

/// A change only a layer surface takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerOp {
    SetKeyboard(KeyboardMode),
    SetExclusiveZone(ExclusiveZone),
    SetSize(LayerSize),
    SetMargin(Margin),
    SetAnchor(Anchor),
    /// `zwlr_layer_surface_v1.set_layer`, v2.
    SetLayer(LayerLevel),
}

/// A change only a toplevel takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowOp {
    /// Start an interactive move (`xdg_toplevel.move`) with the seat's held press on this
    /// surface (`grab`'s rule); ignored, with a log line, without one.
    BeginMove,
    /// Start an interactive resize from `edge` (`xdg_toplevel.resize`), under the same rule as
    /// `BeginMove`.
    BeginResize(ResizeEdge),
    /// `xdg_toplevel.set_maximized` / `unset_maximized`. The compositor answers with a
    /// configure; nothing changes until it does.
    SetMaximized(Maximize),
    /// `xdg_toplevel.set_minimized`. There is no un-minimize request: the compositor (a dock, a
    /// task switcher) restores the window.
    Minimize,
    /// `xdg_toplevel.set_fullscreen` on the compositor's choice of output, or
    /// `unset_fullscreen`.
    SetFullscreen(Fullscreen),
    /// `bardo_window_v1.request_placement` with the seat's latest input serial. Dropped (logged
    /// when there is no input yet) on a compositor without the protocol.
    RequestPlacement(WindowPlacement),
}

impl From<CommonOp> for SurfaceOp {
    fn from(op: CommonOp) -> SurfaceOp {
        SurfaceOp::Common(op)
    }
}

impl From<LayerOp> for SurfaceOp {
    fn from(op: LayerOp) -> SurfaceOp {
        SurfaceOp::Layer(op)
    }
}

impl From<WindowOp> for SurfaceOp {
    fn from(op: WindowOp) -> SurfaceOp {
        SurfaceOp::Window(op)
    }
}

/// Whether applying an op must repaint the surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repaint {
    /// The op changes what the next commit carries (a region, a layer's size or anchor).
    Needed,
    /// A window-management request: the compositor answers with a configure if anything
    /// changes, and that configure repaints. Repainting now would cost an idle frame.
    NotNeeded,
}

impl SurfaceOp {
    /// Whether applying this op repaints the surface.
    #[must_use]
    pub fn repaint(&self) -> Repaint {
        match self {
            SurfaceOp::Window(_) => Repaint::NotNeeded,
            SurfaceOp::Common(_) | SurfaceOp::Layer(_) | SurfaceOp::Reposition(_) => {
                Repaint::Needed
            }
        }
    }
}

/// Whether a surface is on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visibility {
    Shown,
    /// Unmapped, with its document kept alive (a dock hidden over a fullscreen window keeps its
    /// state and its timers).
    Hidden,
}
