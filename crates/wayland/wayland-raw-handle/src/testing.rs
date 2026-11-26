//! For tests: the handles of a test connection's surface, in place of a toolkit's window.
#![expect(
    unsafe_code,
    reason = "raw-window-handle's borrow_raw is unsafe fn; each unsafe block carries its SAFETY comment"
)]

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle, WindowHandle, XlibDisplayHandle,
    XlibWindowHandle,
};
use std::marker::PhantomData;
use std::ptr::NonNull;
use wayland_client::{Connection, Proxy};

/// The handles of a test connection's object, in place of a winit window.
///
/// What a test hands [`adopt`](crate::adopt): a `HasDisplayHandle` and `HasWindowHandle` over the
/// connection and the object. It borrows both, so neither can go away while the handles exist.
#[derive(Debug)]
pub struct Foreign<'a> {
    display: RawDisplayHandle,
    window: RawWindowHandle,
    borrowed: PhantomData<&'a Connection>,
}

impl<'a> Foreign<'a> {
    /// The handles of `window`, an object of `connection`: a surface for a real window. `None`
    /// without the system libwayland.
    #[must_use]
    pub fn of<P: Proxy>(connection: &'a Connection, window: &'a P) -> Option<Foreign<'a>> {
        let display = NonNull::new(connection.backend().display_ptr().cast())?;
        let window = NonNull::new(window.id().as_ptr().cast())?;
        Some(Foreign {
            display: RawDisplayHandle::Wayland(WaylandDisplayHandle::new(display)),
            window: RawWindowHandle::Wayland(WaylandWindowHandle::new(window)),
            borrowed: PhantomData,
        })
    }

    /// The handles of an X11 window.
    #[must_use]
    pub fn x11() -> Foreign<'static> {
        Foreign {
            display: RawDisplayHandle::Xlib(XlibDisplayHandle::new(None, 0)),
            window: RawWindowHandle::Xlib(XlibWindowHandle::new(1)),
            borrowed: PhantomData,
        }
    }
}

impl HasDisplayHandle for Foreign<'_> {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // SAFETY: the connection and the surface are borrowed for as long as this value.
        Ok(unsafe { DisplayHandle::borrow_raw(self.display) })
    }
}

impl HasWindowHandle for Foreign<'_> {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // SAFETY: as for the display handle.
        Ok(unsafe { WindowHandle::borrow_raw(self.window) })
    }
}
