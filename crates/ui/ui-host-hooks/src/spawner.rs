//! The [`Spawner`] every library below the renderer is given.

use std::pin::Pin;

use tokio::runtime::Handle;
use ui_components::spawn::Spawner;

/// The [`Spawner`] every library below the renderer is given: it starts tasks on a Tokio runtime.
#[derive(Debug, Clone)]
pub struct TokioSpawner(Handle);

impl TokioSpawner {
    /// The runtime entered on the calling thread: inside `ui_blitz::launch`, a `Harness`, or a test's own
    /// `#[tokio::test]`.
    ///
    /// # Panics
    /// When no runtime is entered on this thread, a caller contract like Tokio's own.
    #[must_use]
    pub fn current() -> Self {
        TokioSpawner(Handle::current())
    }

    /// The runtime `handle` belongs to, for a program that owns its own (a daemon's).
    #[must_use]
    pub fn on(handle: Handle) -> Self {
        TokioSpawner(handle)
    }
}

impl Spawner for TokioSpawner {
    fn spawn(&self, task: Pin<Box<dyn Future<Output = ()> + Send>>) {
        self.0.spawn(task);
    }
}
