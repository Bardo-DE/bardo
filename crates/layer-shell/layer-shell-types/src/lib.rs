//! The layer-shell host's vocabulary as plain data: ids, geometry, regions, scale, the configs a
//! surface is spawned with, the ops it is told, modifiers and input modality, drags and the
//! session lock. No renderer, GPU, Wayland or UI toolkit type is named here, so a pure crate can
//! take a `LayerConfig` or an `OutputInfo` without linking the host.
//!
//! `layer-shell-hooks` is what a component calls; `layer-shell-host` is the Wayland host that
//! renders the surfaces. Every module is private; the curated root below is the API.

mod blur;
mod drag;
mod focus;
mod geometry;
mod id;
mod layer;
mod lock;
mod modality;
mod modifiers;
mod op;
mod output;
mod placement;
mod popup;
mod region;
mod scale;
mod scroll_state;
mod tick;
mod toplevel;
mod wheel;
mod window_placement;

pub use blur::BlurSpec;
pub use drag::{BadMime, DragAction, DragActions, DragOutcome, DragPayload, DragRefused, Mime};
pub use focus::KeyboardFocus;
pub use geometry::{Margin, Point, enclosing_rect, inflate, px_of};
pub use id::{OutputId, SurfaceId, SurfaceName};
pub use layer::{
    Anchor, ExclusiveZone, KeyboardMode, LayerConfig, LayerExtent, LayerLevel, LayerSize,
    OutputTarget,
};
pub use lock::{Ended, LockConfig, LockError, LockId, LockPhase, LockState};
pub use modality::InputModality;
pub use modifiers::{LockKey, Modifier, Modifiers};
pub use op::{CommonOp, LayerOp, Repaint, SurfaceOp, Visibility, WindowOp};
pub use output::OutputInfo;
pub use placement::PopupPlacement;
pub use popup::{
    PopupAdjust, PopupAnchor, PopupConfig, PopupEdge, PopupGrab, PopupOffset, PopupSize,
    offset_rect,
};
pub use region::{ElementId, ElementRegion, Px, Rect, RegionSpec, RoundedRect, rounded_strips};
pub use scale::{
    BufferMapping, MAX_SURFACE_EXTENT, Scale120, Size, buffer_mapping, buffer_size,
    logical_to_physical, wire_extent,
};
pub use scroll_state::{ScrollExtent, ScrollPhase, ScrollPoint, ScrollState};
pub use tick::FrameTick;
pub use toplevel::{
    Activation, Decorations, Fullscreen, Maximize, ResizeEdge, ResizeMargin, Tile, TileEdges,
    ToplevelConfig, ToplevelState,
};
pub use wheel::{WheelEvent, WheelMotion, WheelPhase, WheelSource};
pub use window_placement::{PlacementSupport, WindowPlacement};
