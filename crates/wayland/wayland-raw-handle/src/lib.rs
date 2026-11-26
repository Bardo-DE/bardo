//! Wayland objects that another library owns, and raw-window-handle targets over our own.
//!
//! libwayland hands out its objects as raw pointers. Two jobs need them, and both are done here
//! so that no other crate writes `unsafe`:
//!
//! - [`adopt`] takes the display and surface handles of a window that winit (or another toolkit)
//!   owns, and gives back a [`Adopted`]: a [`wayland_client::Connection`] on the same display and
//!   the window's [`wayland_client::protocol::wl_surface::WlSurface`], so a protocol request can
//!   name the surface of a window that already exists.
//! - [`SurfaceTarget`] (and, with the `wgpu` feature, [`wgpu_surface`] and [`WaylandDisplay`])
//!   turn our own connection and surface into raw-window-handle handles and a wgpu surface.
//!
//! Each `unsafe` block says why it is sound. No function takes a raw pointer: the pointers come
//! from raw-window-handle's handles and, with the `winit` feature, from the winit window itself.

mod adopt;
mod target;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
#[cfg(feature = "wgpu")]
mod wgpu_surface;

pub use adopt::{AdoptError, Adopted, adopt};
pub use target::SurfaceTarget;
#[cfg(feature = "wgpu")]
pub use wgpu_surface::{SurfaceError, WaylandDisplay, wgpu_surface};
