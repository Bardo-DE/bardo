//! What a popup asks for: its anchor, size, edge, gravity, adjustment, grab and offset.

use crate::region::{ElementId, Px};

/// Where a popup is anchored: an explicit rect, or a DOM element resolved after layout.
///
/// Not `Copy`: `ElementId` owns a `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupAnchor {
    Rect,
    Element(ElementId),
}

/// How a popup's size is determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupSize {
    /// Use the size passed to `positioner_params` as-is.
    Fixed,
    /// Clamp the passed-in size to `max` on each axis (content sizes itself, up to a cap).
    FitContent { max: Px },
}

/// Which point of the anchor rectangle the popup is positioned relative to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupEdge {
    None,
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    BottomLeft,
    TopRight,
    BottomRight,
}

/// The `xdg_positioner` constraint-adjustment flags: how the compositor may move or resize the
/// popup if its unadjusted position would be (partly) off-screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PopupAdjust {
    pub slide_x: bool,
    pub slide_y: bool,
    pub flip_x: bool,
    pub flip_y: bool,
    pub resize_x: bool,
    pub resize_y: bool,
}

impl PopupAdjust {
    /// No adjustment: the popup stays exactly where positioned, even off-screen.
    pub const NONE: PopupAdjust = PopupAdjust {
        slide_x: false,
        slide_y: false,
        flip_x: false,
        flip_y: false,
        resize_x: false,
        resize_y: false,
    };

    /// Slide on both axes, flip on both axes, and never resize: the common "menu" behaviour.
    pub const SLIDE_AND_FLIP: PopupAdjust = PopupAdjust {
        slide_x: true,
        slide_y: true,
        flip_x: true,
        flip_y: true,
        resize_x: false,
        resize_y: false,
    };
}

/// Whether a popup takes an exclusive input grab (dismissing it, and any popups nested inside
/// it, when the compositor sees a click outside the stack).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupGrab {
    Ungrabbed,
    Grabbed,
}

/// A shift of the popup from where its anchor edge and gravity put it, in the parent's logical
/// pixels: design/13 §13.3.2 places a bar menu at (-8, +6) from its title button. Applied to
/// the anchor rectangle before it is sent, so it moves with the anchor and survives a flip
/// or a reposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PopupOffset {
    pub dx: Px,
    pub dy: Px,
}

impl PopupOffset {
    /// No shift: the popup sits exactly where its anchor edge and gravity put it.
    pub const ZERO: PopupOffset = PopupOffset { dx: 0, dy: 0 };
}

/// How a popup should be positioned and behave, independent of any particular anchor rect or
/// content size (those are resolved separately, after layout, and passed to
/// `positioner_params`).
///
/// Not `Copy`: `anchor` may own an `ElementId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupConfig {
    pub anchor: PopupAnchor,
    pub size: PopupSize,
    pub edge: PopupEdge,
    pub gravity: PopupEdge,
    pub adjust: PopupAdjust,
    pub grab: PopupGrab,
    /// Where the popup sits relative to its anchor; `PopupOffset::ZERO` for none.
    pub offset: PopupOffset,
}
