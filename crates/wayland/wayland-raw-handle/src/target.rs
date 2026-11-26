//! A connection and one of its surfaces as raw-window-handle handles.
#![expect(
    unsafe_code,
    reason = "raw-window-handle's borrow_raw is unsafe fn; each unsafe block carries its SAFETY comment"
)]

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle, WindowHandle,
};
use std::ffi::c_void;
use std::ptr::NonNull;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Proxy};

/// The `wl_display` pointer of `connection`, or `None` when wayland-backend is not using the
/// system libwayland.
pub(crate) fn display_pointer(connection: &Connection) -> Option<NonNull<c_void>> {
    NonNull::new(connection.backend().display_ptr().cast::<c_void>())
}

/// The display handle of `connection`, for as long as `connection` is borrowed.
pub(crate) fn display_handle(connection: &Connection) -> Result<DisplayHandle<'_>, HandleError> {
    let display = display_pointer(connection).ok_or(HandleError::Unavailable)?;
    let raw = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(display));
    // SAFETY: the pointer is the `wl_display*` of `connection`, which is open for as long as
    // the borrow of `connection` that the handle carries.
    Ok(unsafe { DisplayHandle::borrow_raw(raw) })
}

/// A connection and one of its surfaces, as the window and display handles a graphics API wants.
///
/// It holds clones of both, so the display stays connected and the surface's proxy stays alive for
/// as long as this value, and so as long as anything that owns it (a wgpu surface made from it).
/// Do not send `wl_surface.destroy` on the surface while a graphics surface made from this target
/// is alive.
#[derive(Debug)]
pub struct SurfaceTarget {
    connection: Connection,
    surface: WlSurface,
}

impl SurfaceTarget {
    /// The target of `surface`, an object of `connection`.
    #[must_use]
    pub fn new(connection: Connection, surface: WlSurface) -> SurfaceTarget {
        SurfaceTarget {
            connection,
            surface,
        }
    }
}

impl HasDisplayHandle for SurfaceTarget {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        display_handle(&self.connection)
    }
}

impl HasWindowHandle for SurfaceTarget {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let surface = NonNull::new(self.surface.id().as_ptr().cast::<c_void>())
            .ok_or(HandleError::Unavailable)?;
        let raw = RawWindowHandle::Wayland(WaylandWindowHandle::new(surface));
        // SAFETY: the pointer is the `wl_proxy*` of `self.surface`, which this value holds alive
        // (and on its connection, also held) for as long as the borrow of `self` that the handle
        // carries. A destroyed surface has a null pointer, which is refused above.
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}
