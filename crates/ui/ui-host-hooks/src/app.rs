//! What a component may ask of the running app, without naming the renderer that runs it.

use std::sync::Arc;

use dioxus::prelude::*;

/// The app's event loop has ended: a request to it is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the app's event loop has ended")]
pub struct AppEndedError;

/// Ends the app: what a window's root reads with [`use_app_control`]. The renderer's own handle
/// (`ui_blitz::AppHandle`) provides it, and does the rest of what an app asks from outside its
/// documents.
#[derive(Clone)]
pub struct AppControl {
    quit: Arc<dyn Fn() -> Result<(), AppEndedError> + Send + Sync>,
}

impl AppControl {
    /// A control whose [`quit`](AppControl::quit) calls `quit`.
    pub fn new(quit: impl Fn() -> Result<(), AppEndedError> + Send + Sync + 'static) -> Self {
        AppControl {
            quit: Arc::new(quit),
        }
    }

    /// End the app: close its windows and leave the event loop.
    ///
    /// # Errors
    ///
    /// [`AppEndedError`] when the loop has already ended, which is the same as the windows being closed.
    pub fn quit(&self) -> Result<(), AppEndedError> {
        (self.quit)()
    }
}

impl std::fmt::Debug for AppControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppControl").finish_non_exhaustive()
    }
}

/// The running app's control, or `None` in a document that is not an app's window (the test
/// harness's, a snapshot's).
pub fn use_app_control() -> Option<AppControl> {
    use_hook(try_consume_context::<AppControl>)
}
