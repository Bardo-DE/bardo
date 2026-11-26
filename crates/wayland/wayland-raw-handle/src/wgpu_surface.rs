//! A wgpu surface over a Wayland connection's `wl_surface`.

use crate::target::{SurfaceTarget, display_handle, display_pointer};
use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle};
use std::sync::Arc;
use wayland_client::Connection;
use wayland_client::protocol::wl_surface::WlSurface;

/// Why a wgpu surface could not be made.
#[derive(Debug, thiserror::Error)]
pub enum SurfaceError {
    /// The connection has no `wl_display` pointer (wayland-backend without the system library).
    #[error("the connection has no wl_display pointer")]
    NoDisplay,
    /// The surface has no pointer: it was destroyed.
    #[error("the wl_surface has been destroyed")]
    NoSurface,
    /// wgpu refused the handles.
    #[error("wgpu cannot make a surface from the handles: {0}")]
    Create(#[source] wgpu::CreateSurfaceError),
}

/// A wgpu surface presenting into `surface`, an object of `connection`.
///
/// The surface owns clones of both (see [`SurfaceTarget`]), so the display stays connected until it
/// drops. `instance` must have been made with a [`WaylandDisplay`] of the same connection
/// when it uses GL, which needs the display at instance creation.
///
/// The caller must not send `wl_surface.destroy` before the returned surface is dropped.
///
/// # Errors
///
/// Returns a [`SurfaceError`] when either pointer is null or wgpu refuses the handles.
pub fn wgpu_surface(
    instance: &wgpu::Instance,
    connection: &Connection,
    surface: &WlSurface,
) -> Result<wgpu::Surface<'static>, SurfaceError> {
    let target = SurfaceTarget::new(connection.clone(), surface.clone());
    if display_pointer(connection).is_none() {
        return Err(SurfaceError::NoDisplay);
    }
    if raw_window_handle::HasWindowHandle::window_handle(&target).is_err() {
        return Err(SurfaceError::NoSurface);
    }
    instance
        .create_surface(Arc::new(target))
        .map_err(SurfaceError::Create)
}

/// A connection's `wl_display` as a display handle, for wgpu's `InstanceDescriptor` (GL needs the
/// display when the instance is made). It holds the connection, so the display outlives it.
#[derive(Debug)]
pub struct WaylandDisplay(Connection);

impl WaylandDisplay {
    /// The display of `connection`; `None` when wayland-backend is not using the system libwayland.
    #[must_use]
    pub fn new(connection: &Connection) -> Option<WaylandDisplay> {
        display_pointer(connection).map(|_| WaylandDisplay(connection.clone()))
    }
}

impl HasDisplayHandle for WaylandDisplay {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        display_handle(&self.0)
    }
}
