//! A window's frame effects, asked of the compositor: once the window exists, on a Wayland
//! compositor that offers `bardo_window_v1`, the compositor is told to draw the shadow and clip
//! the corners at the radius of the window's own drawn edge (`crate::bardo_window`). Anywhere else
//! (X11, GNOME, KDE, macOS, a build without `desktop`) nothing is asked and nothing is shown.

use dioxus_native::winit::window::Window;

/// What one window has told the compositor about its frame.
#[derive(Debug, Default)]
pub(crate) struct WindowFrame {
    #[cfg(all(target_os = "linux", feature = "desktop"))]
    state: State,
}

/// How far the asking has got.
#[cfg(all(target_os = "linux", feature = "desktop"))]
#[derive(Debug, Default)]
enum State {
    /// Not tried yet.
    #[default]
    Unasked,
    /// The compositor has the window's object, kept until the window goes.
    Told(crate::bardo_window::BardoWindow),
    /// No Wayland window, or no such global: left as it was.
    Declined,
}

#[cfg(all(target_os = "linux", feature = "desktop"))]
impl WindowFrame {
    /// Tell the compositor, once, and again if the radius has changed. Cheap after the first.
    pub(crate) fn tell(&mut self, window: &dyn Window) {
        let radius = crate::bardo_window::window_radius();
        match &mut self.state {
            State::Unasked => {
                self.state = match attach(window) {
                    Some(mut told) => match told.set_radius(radius) {
                        Ok(()) => State::Told(told),
                        Err(_) => State::Declined,
                    },
                    None => State::Declined,
                };
            }
            State::Told(told) => {
                if told.set_radius(radius).is_err() {
                    self.state = State::Declined;
                }
            }
            State::Declined => {}
        }
    }
}

/// The window's object, if it is a Wayland window and the compositor offers the global.
#[cfg(all(target_os = "linux", feature = "desktop"))]
fn attach(window: &dyn Window) -> Option<crate::bardo_window::BardoWindow> {
    use crate::{bardo_window, wayland_surface::adopt};
    use dioxus_native::winit::platform::wayland::WindowExtWayland;
    use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
    let toplevel = window.xdg_toplevel()?;
    let display = window.display_handle().ok()?;
    let handle = window.window_handle().ok()?;
    let adopted = adopt(display, handle).ok()?;
    bardo_window::attach(&adopted, toplevel).ok()
}

/// Elsewhere, or without `desktop`, there is no compositor to ask.
#[cfg(not(all(target_os = "linux", feature = "desktop")))]
impl WindowFrame {
    /// Nothing to tell.
    pub(crate) fn tell(&mut self, _window: &dyn Window) {}
}
