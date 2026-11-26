//! A window's Wayland connection and surface, borrowed from its raw-window-handle handles.
#![expect(
    unsafe_code,
    reason = "libwayland pointers handed over by another library; each unsafe block carries its SAFETY comment"
)]

use raw_window_handle::{DisplayHandle, RawDisplayHandle, RawWindowHandle, WindowHandle};
use std::ffi::c_void;
use std::marker::PhantomData;
use std::ptr::NonNull;
use wayland_backend::client::{Backend, InvalidId, ObjectId};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Proxy};

/// Why a window's connection and surface could not be borrowed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AdoptError {
    /// The window or the display is not Wayland's.
    #[error("the window is not a Wayland window")]
    ForeignWindow,
    /// An object handed to [`Adopted::share`] lives on another display, or its connection is gone.
    #[error("the {interface} is not on the window's display")]
    OtherDisplay {
        /// The protocol interface of the object, such as `xdg_toplevel`.
        interface: &'static str,
    },
    /// libwayland does not know the pointer as a live object of the class asked for.
    #[error("the pointer is not a live {interface}")]
    DeadObject {
        /// The protocol interface that was asked for, such as `wl_surface`.
        interface: &'static str,
        /// What the connection said.
        #[source]
        source: InvalidId,
    },
}

/// A window's `wl_surface` and a connection over the display it lives on.
///
/// Both are borrowed from the window's handles: they cannot outlive `'a`, and dropping this
/// closes nothing the window's owner holds (only the proxies and the event queue the connection
/// created).
pub struct Adopted<'a> {
    connection: Connection,
    surface: WlSurface,
    /// What the pointers were borrowed from.
    handles: PhantomData<(DisplayHandle<'a>, WindowHandle<'a>)>,
}

impl std::fmt::Debug for Adopted<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Adopted")
            .field("surface", &self.surface.id())
            .finish()
    }
}

impl Adopted<'_> {
    /// The connection to make requests on, sharing the window's display.
    #[must_use]
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    /// The window's surface.
    #[must_use]
    pub fn surface(&self) -> &WlSurface {
        &self.surface
    }

    /// The same protocol object as `proxy`, as an `I` on this connection, so a request made here
    /// can name it.
    ///
    /// # Errors
    ///
    /// Returns [`AdoptError::OtherDisplay`] when `proxy` is not on the window's display (or its
    /// connection is gone), and [`AdoptError::DeadObject`] when it is no longer alive.
    pub fn share<I: Proxy + 'static>(&self, proxy: &I) -> Result<I, AdoptError> {
        let other = AdoptError::OtherDisplay {
            interface: I::interface().name,
        };
        let backend = proxy.backend().upgrade().ok_or(other.clone())?;
        if backend.display_ptr() != self.connection.backend().display_ptr() {
            return Err(other);
        }
        let pointer = NonNull::new(proxy.id().as_ptr().cast()).ok_or(other)?;
        object(&self.connection, pointer)
    }

    /// The `xdg_toplevel` of a winit window, as an `I` (wayland-protocols' `XdgToplevel`): `None`
    /// when the window is not a Wayland one.
    ///
    /// `window` must be the window these handles came from. winit reports the toplevel as a
    /// `wl_proxy*` of its connection, live for the window's life, and libwayland checks that its
    /// class is `I`. Dropping the result destroys no protocol object.
    ///
    /// # Errors
    ///
    /// Returns [`AdoptError::DeadObject`] when libwayland does not know the toplevel as an `I`.
    #[cfg(feature = "winit")]
    pub fn xdg_toplevel<I: Proxy + 'static>(
        &self,
        window: &dyn winit_core::window::Window,
    ) -> Result<Option<I>, AdoptError> {
        use winit_wayland::WindowExtWayland;
        window
            .xdg_toplevel()
            .map(|pointer| object(&self.connection, pointer))
            .transpose()
    }
}

/// The object at `pointer`, as an `I` on `connection`.
fn object<I: Proxy + 'static>(
    connection: &Connection,
    pointer: NonNull<c_void>,
) -> Result<I, AdoptError> {
    let dead = |source| AdoptError::DeadObject {
        interface: I::interface().name,
        source,
    };
    // SAFETY: `pointer` is a `wl_proxy*` of a live object of the display `connection` is on: the
    // window handle's surface (raw-window-handle's contract), the toplevel winit reports for the
    // same window, or a live proxy whose display `share` compared; `from_ptr` checks the proxy's class against `I`
    // and answers an error, not a bad object, on a mismatch. The id it makes owns nothing:
    // dropping it destroys no protocol object.
    let id =
        unsafe { ObjectId::from_ptr(I::interface(), pointer.as_ptr().cast()) }.map_err(dead)?;
    I::from_id(connection, id).map_err(dead)
}

/// The connection and surface behind a Wayland window's handles.
///
/// # Errors
///
/// Returns [`AdoptError::ForeignWindow`] when either handle is not Wayland's, and
/// [`AdoptError::DeadObject`] when libwayland does not know the surface as a `wl_surface`.
pub fn adopt<'a>(
    display: DisplayHandle<'a>,
    window: WindowHandle<'a>,
) -> Result<Adopted<'a>, AdoptError> {
    let (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(window)) =
        (display.as_raw(), window.as_raw())
    else {
        return Err(AdoptError::ForeignWindow);
    };
    // SAFETY: `display` is the `wl_display*` of a live connection: raw-window-handle's contract
    // for the `DisplayHandle<'a>` it came from, which `Adopted<'a>` keeps borrowed for as long
    // as the backend exists. A foreign display is never disconnected by the backend: dropping it
    // destroys only the proxies and the event queue created through it.
    let backend = unsafe { Backend::from_foreign_display(display.display.as_ptr().cast()) };
    let connection = Connection::from_backend(backend);
    // `window.surface` is a `wl_surface*` of that display, live for the `WindowHandle<'a>` it
    // came from, which `Adopted<'a>` keeps borrowed.
    let surface = object::<WlSurface>(&connection, window.surface)?;
    Ok(Adopted {
        connection,
        surface,
        handles: PhantomData,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Foreign;
    use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

    #[test]
    fn an_x11_window_is_not_adopted() {
        let window = Foreign::x11();
        let adopted = adopt(
            window.display_handle().expect("a display handle"),
            window.window_handle().expect("a window handle"),
        );
        assert_eq!(adopted.err(), Some(AdoptError::ForeignWindow));
    }
}
