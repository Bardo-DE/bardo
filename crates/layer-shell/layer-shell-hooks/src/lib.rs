//! What a component of a layer-shell host surface calls: the `SurfaceHandle` in its root context
//! and the hooks that read it (`use_surface`, `use_surface_scale`, `use_input_modality`,
//! `use_modifiers`, `use_toplevel_state`, `use_frame_tick`, `use_wheel`, `use_scroll`, `use_lock`,
//! `use_lock_phase`, `use_watch`), and the `HostRoot` a surface or popup renders.
//!
//! This crate names no renderer, GPU or Wayland type, so a UI library links the hooks without the
//! host. The seam to the host that renders the surfaces is plain data and queues: a handle pushes
//! [`Request`]s into a [`RequestQueue`] the host drains, and reads [`Live`] cells, [`FrameTicks`],
//! a [`WheelFeed`] and a [`ScrollBoard`] the host publishes into. The vocabulary is
//! `layer-shell-types`'s; the host is `layer-shell-host`.

mod drag;
mod grab;
mod handle;
mod ids;
mod live;
mod lock_handle;
mod native;
mod request;
mod root;
mod scroll_watch;
mod tick;
mod watch;
mod wheel_feed;

pub use drag::{DragIcon, DragSpec};
pub use grab::{GrabError, GrabRecord, PointerFacts, Press, grab_serial};
pub use handle::{
    HandleParts, RoleKind, SurfaceHandle, use_frame_tick, use_input_modality, use_modifiers,
    use_surface, use_surface_scale, use_toplevel_state, use_wheel,
};
pub use ids::SurfaceIds;
pub use live::{Live, NextValue};
pub use lock_handle::{LockHandle, use_lock, use_lock_phase};
pub use native::NativeCell;
pub use request::{Request, RequestQueue};
pub use root::HostRoot;
pub use scroll_watch::{ScrollBoard, use_scroll};
pub use tick::{FrameDrive, FrameTicks, NextTick, pacing, tick_after};
pub use watch::use_watch;
pub use wheel_feed::{NextWheel, WheelFeed, WheelListener};
