//! What can go wrong launching the app.

/// Why [`launch`](crate::launch) or [`launch_idle`](crate::launch_idle) could not run the app.
#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    /// No runtime for the app's `tokio::spawn` calls (and none was given with
    /// [`AppConfig::with_runtime`](crate::AppConfig::with_runtime)).
    #[error("the host's tokio runtime could not be started: {0}")]
    Runtime(#[from] std::io::Error),
    /// The windowing event loop could not run: a broken host, with no window to show anything in.
    #[error("the window's event loop could not run: {0}")]
    EventLoop(#[source] Box<dyn std::error::Error + Send + Sync>),
}
