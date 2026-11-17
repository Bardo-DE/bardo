//! `SurfaceHandle`: what a component uses to change its own surface. Provided as a root
//! context of every surface's `VirtualDom`; read it with [`use_surface`].
//!
//! See FINDINGS F31, F43, F46, F60.

mod hooks;
mod surface;
mod window;

pub use hooks::{
    use_frame_tick, use_input_modality, use_modifiers, use_surface, use_surface_scale,
    use_toplevel_state, use_wheel,
};

use layer_shell_types::{
    InputModality, Modifiers, PlacementSupport, Scale120, SurfaceId, SurfaceOp, ToplevelState,
};

use crate::grab::GrabRecord;
use crate::ids::SurfaceIds;
use crate::live::Live;
use crate::native::NativeCell;
use crate::request::{Request, RequestQueue};
use crate::scroll_watch::ScrollBoard;
use crate::tick::FrameTicks;
use crate::wheel_feed::WheelFeed;

/// Which role a surface has, without its objects: what a `SurfaceHandle` knows of its surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoleKind {
    Layer,
    Popup,
    Toplevel,
    DragIcon,
    Lock,
}

/// One surface, as seen from inside its document.
#[derive(Debug, Clone)]
pub struct SurfaceHandle {
    id: SurfaceId,
    kind: RoleKind,
    requests: RequestQueue,
    ids: SurfaceIds,
    ticks: FrameTicks,
    scale: Live<Scale120>,
    toplevel: Live<ToplevelState>,
    grab: GrabRecord,
    scrolls: ScrollBoard,
    wheels: WheelFeed,
    modifiers: Live<Modifiers>,
    modality: Live<InputModality>,
    native: NativeCell,
    placement: PlacementSupport,
}

/// The host's shared state a `SurfaceHandle` reads, besides its queue: built by the host when
/// it creates the surface.
#[derive(Debug, Clone)]
pub struct HandleParts {
    pub requests: RequestQueue,
    pub ids: SurfaceIds,
    pub ticks: FrameTicks,
    pub scale: Live<Scale120>,
    pub toplevel: Live<ToplevelState>,
    pub grab: GrabRecord,
    pub scrolls: ScrollBoard,
    pub wheels: WheelFeed,
    pub modifiers: Live<Modifiers>,
    /// The seat's, shared by every surface (not a per-surface copy like `modifiers`).
    pub modality: Live<InputModality>,
    /// This surface's platform object (a `wl_surface`), refreshed when the host rebuilds the role.
    pub native: NativeCell,
    /// Whether the compositor takes placement requests (the same for every surface).
    pub placement: PlacementSupport,
}

impl SurfaceHandle {
    /// The handle for surface `id` of role `kind`.
    pub fn new(id: SurfaceId, kind: RoleKind, parts: HandleParts) -> SurfaceHandle {
        let HandleParts {
            requests,
            ids,
            ticks,
            scale,
            toplevel,
            grab,
            scrolls,
            wheels,
            modifiers,
            modality,
            native,
            placement,
        } = parts;
        SurfaceHandle {
            id,
            kind,
            requests,
            ids,
            ticks,
            scale,
            toplevel,
            grab,
            scrolls,
            wheels,
            modifiers,
            modality,
            native,
            placement,
        }
    }

    /// This surface's id.
    #[must_use]
    pub fn id(&self) -> SurfaceId {
        self.id
    }

    /// This surface's scale right now, without subscribing to changes: for an event handler
    /// or a task. A component that draws for the scale reads [`use_surface_scale`] instead.
    #[must_use]
    pub fn current_scale(&self) -> Scale120 {
        self.scale.get()
    }

    /// This surface's platform object as a `T`, for a protocol the host does not wrap (the Wayland
    /// host's is a `wl_surface`, for a `quire_*` object the app's own code creates on the shell's
    /// connection). It is replaced when the surface is hidden and shown again, so read it again
    /// after `HostEvent::SurfaceMapped`. `None` for a handle that has no surface behind it (one
    /// built without a connection), or when the object is not a `T`.
    #[must_use]
    pub fn native<T: Clone + 'static>(&self) -> Option<T> {
        self.native.get()
    }

    /// Ask the host for something only it defines (a programmatic scroll, say): `payload` is
    /// queued as `Request::Host` and the host downcasts it.
    pub fn send_host<T: 'static>(&self, payload: T) {
        self.requests.push(Request::Host(Box::new(payload)));
    }

    /// Any op, as `HostCtx::apply` would apply it.
    pub fn apply(&self, op: impl Into<SurfaceOp>) {
        self.requests.push(Request::Apply {
            id: self.id,
            op: op.into(),
        });
    }

    /// The seat's input modality right now, without subscribing: for an event handler. A
    /// component that draws from it reads [`use_input_modality`].
    #[must_use]
    pub fn current_input_modality(&self) -> InputModality {
        self.modality.get()
    }

    /// The scrollers this surface's components watch (`use_scroll`).
    pub(crate) fn scroll_board(&self) -> ScrollBoard {
        self.scrolls.clone()
    }
}
