//! The window seam's vocabulary: what a frame asks of its host (`ResizeEdge`, `Zoom`, `WindowTile`)
//! and what the host reports back (`WindowState`, `TileError`).
//!
//! Closed sets, so a host that is not ui-blitz (layer-shell-host's `SurfaceHandle`) maps each one
//! exhaustively.

use ui_core::word::Word;

/// The edge or corner an interactive resize grabs (the xdg-shell `resize_edge` set, winit's
/// `ResizeDirection`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub enum ResizeEdge {
    /// The top edge.
    Top,
    /// The bottom edge.
    Bottom,
    /// The left edge.
    Left,
    /// The right edge.
    Right,
    /// The top-left corner.
    TopLeft,
    /// The top-right corner.
    TopRight,
    /// The bottom-left corner.
    BottomLeft,
    /// The bottom-right corner.
    BottomRight,
}

/// What the zoom control asks for (design/13-BEHAVIOUR-menus-windows.md section 13.3.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Zoom {
    /// Maximized becomes restored and anything else maximized: the green light and a
    /// double-click on the titlebar.
    Toggle,
    /// Fill the work area (the tiling menu's Fill).
    Maximize,
    /// Back to the size and place it had before.
    Restore,
}

/// A step of the page's zoom, as the standard Bigger, Smaller and Actual Size ask for it
/// (Primary with `+`, `-` and `0`): everything in the window drawn larger or smaller, laid out
/// again at its new size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageZoom {
    /// One step larger.
    Bigger,
    /// One step smaller.
    Smaller,
    /// Back to the page's own size.
    ActualSize,
}

/// A placement the tiling menu offers (macOS Sequoia's Window > Move & Resize). Named
/// `WindowTile` because `ui_components::MenuTile` is a menu row's leading tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub enum WindowTile {
    /// The whole work area: the same as `Zoom::Maximize`.
    Fill,
    /// The left half of the output.
    LeftHalf,
    /// The right half of the output.
    RightHalf,
    /// The current size, centred on the output.
    Centre,
}

/// A host did not place its window: the platform does not let a client place itself there.
/// On Wayland a toplevel cannot read or set its own position (FINDINGS "Window frame").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("the platform does not let this window place itself there")]
pub struct TileError;

/// What the host last reported about its window. The default is what a window with no host
/// looks like: its own size, not fullscreen, active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowState {
    /// Whether the window fills its output's work area (zoomed).
    pub maximized: bool,
    /// Whether the window covers its whole output with no frame.
    pub fullscreen: bool,
    /// Whether the window is the one the keyboard goes to (the xdg `activated` state): its
    /// lights are coloured at rest; behind another they are grey until the pointer is over them.
    pub activated: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        WindowState {
            maximized: false,
            fullscreen: false,
            activated: true,
        }
    }
}

impl WindowState {
    /// Whether the frame may start a move or a resize: not while the window fills its output
    /// (a maximized window's place is the compositor's, and a drag would fight it).
    #[must_use]
    pub fn movable(self) -> bool {
        !self.maximized && !self.fullscreen
    }

    /// The `data-window` word the frame is drawn by.
    pub(crate) fn slug(self) -> &'static str {
        match (self.fullscreen, self.maximized) {
            (true, _) => "fullscreen",
            (false, true) => "maximized",
            (false, false) => "normal",
        }
    }

    /// Whether the window is the one the person works in, as the shared vocabulary says it.
    pub(crate) fn activity(self) -> bool {
        self.activated
    }
}
