//! What a `SurfaceHandle` asks of the Wayland host beyond the portable hooks: a programmatic
//! scroll and the surface's `wl_surface`. Both are traits implemented here for
//! `layer_shell_hooks::SurfaceHandle`, whose request queue and native cell carry them
//! type-erased (`SurfaceHandle::send_host`, `SurfaceHandle::native`), so the hooks crate names
//! neither the scroll engine nor Wayland.

use blitz_embed::scroll::cmd::ScrollCmd;
use layer_shell_hooks::SurfaceHandle;
use layer_shell_types::SurfaceId;
use wayland_client::protocol::wl_surface::WlSurface;

/// A programmatic scroll queued by [`SurfaceScroll::scroll`], answered by the host's engine.
#[derive(Debug)]
pub(crate) struct ScrollRequest {
    pub(crate) id: SurfaceId,
    pub(crate) cmd: ScrollCmd,
}

/// Scrolling a container of a surface from code.
pub trait SurfaceScroll {
    /// Scroll a scroll container of this surface through the host's engine (the smooth rule,
    /// or at once): the way to scroll from code, since Blitz's own `scroll_to` would fight the
    /// engine (design/11 §11.3.11).
    fn scroll(&self, cmd: ScrollCmd);
}

impl SurfaceScroll for SurfaceHandle {
    fn scroll(&self, cmd: ScrollCmd) {
        self.send_host(ScrollRequest { id: self.id(), cmd });
    }
}

/// The Wayland object behind a surface.
pub trait SurfaceWayland {
    /// This surface's `wl_surface`, for a protocol the host does not wrap (a `quire_*` object
    /// the app's own code creates on the shell's connection). It is replaced when the surface
    /// is hidden and shown again, so read it again after `HostEvent::SurfaceMapped`. `None` for
    /// a handle that has no surface behind it (one built without a connection).
    fn wl_surface(&self) -> Option<WlSurface>;
}

impl SurfaceWayland for SurfaceHandle {
    fn wl_surface(&self) -> Option<WlSurface> {
        self.native::<WlSurface>()
    }
}
