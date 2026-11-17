//! What a component may ask of the window it renders in, without naming the renderer: close it,
//! raise it.

use std::rc::Rc;

use dioxus::prelude::*;

/// Closes or raises the window: what a window's root reads with [`use_window_control`]. The
/// renderer's own handle (`ui_blitz::WindowHandle`) provides it.
#[derive(Clone)]
pub struct WindowControl {
    close: Rc<dyn Fn()>,
    focus: Rc<dyn Fn(Option<String>)>,
}

impl WindowControl {
    /// A control whose [`close`](WindowControl::close) calls `close` and whose
    /// [`focus_with_token`](WindowControl::focus_with_token) calls `focus`.
    pub fn new(close: impl Fn() + 'static, focus: impl Fn(Option<String>) + 'static) -> Self {
        WindowControl {
            close: Rc::new(close),
            focus: Rc::new(focus),
        }
    }

    /// Close the window: it leaves the screen and its document is dropped. Closing one already
    /// closed does nothing.
    pub fn close(&self) {
        (self.close)();
    }

    /// Raise the window and give it the keyboard with an activation token: the one a launcher, a
    /// notification click or a second launch handed the running app. On Wayland the token is the
    /// only way to raise a window that exists; with `None`, or an empty token, the window is
    /// raised the way the platform allows. A token is good once.
    pub fn focus_with_token(&self, token: Option<String>) {
        (self.focus)(token);
    }
}

impl std::fmt::Debug for WindowControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowControl").finish_non_exhaustive()
    }
}

/// The control of the window this component renders in, or `None` in a document that is not a
/// window (the test harness's, a shell surface).
pub fn use_window_control() -> Option<WindowControl> {
    use_hook(try_consume_context::<WindowControl>)
}
