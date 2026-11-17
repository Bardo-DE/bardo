//! The one way library code starts a task that outlives its caller.
//!
//! It hands the future to a [`Spawner`] it was given, and never names a runtime. The program's
//! launcher owns the runtime and provides the implementor; a test drives an inline one.

use std::pin::Pin;

/// Runs tasks on the program's runtime.
pub trait Spawner: Send + Sync {
    /// Start `task`; it runs until it finishes or the runtime stops.
    fn spawn(&self, task: Pin<Box<dyn Future<Output = ()> + Send>>);
}
