//! What a component sees of a scroller it watches with `use_scroll`.

/// A scroll position in logical px; raw, so out of range while rubber-banding.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScrollPoint {
    pub x: f64,
    pub y: f64,
}

/// A scroller's content and viewport sizes, logical px.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScrollExtent {
    pub content_width: f64,
    pub content_height: f64,
    pub viewport_width: f64,
    pub viewport_height: f64,
}

/// What is moving a scroller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollPhase {
    #[default]
    Idle,
    /// Fingers are down on it.
    Tracking,
    /// It glides after a flick.
    Momentum,
    /// It springs back from past an edge.
    Rebound,
    /// A key, wheel or command animates it.
    Animating,
}

/// One scroller as a component sees it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScrollState {
    pub offset: ScrollPoint,
    pub extent: ScrollExtent,
    pub phase: ScrollPhase,
}
