//! The D-Bus connections casement holds (`compositor_app::IpcState::handles`).

/// The live D-Bus connections: the Debug1 server under `debug-inject` (P0) and
/// `org.bardo.Compositor1` on the session bus (P2-Z).
#[derive(Debug, Default)]
pub struct IpcHandles {
    /// The private bus connection serving Debug1, once started.
    #[cfg(feature = "debug-inject")]
    pub debug: Option<zbus::Connection>,
    /// The session bus connection that owns `org.bardo.Compositor1`, once it is served. Kept
    /// so the name is held for as long as the compositor runs.
    pub compositor: Option<zbus::Connection>,
}

impl IpcHandles {
    /// No connection yet.
    #[must_use]
    pub fn none() -> IpcHandles {
        IpcHandles::default()
    }
}
