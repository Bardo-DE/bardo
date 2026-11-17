//! The errors the service reports. Library paths return these; nothing panics on a bad request.

/// Why the portal could not start or finish something.
#[derive(Debug, thiserror::Error)]
pub enum TransomError {
    /// The bus refused us (no session bus, the name is taken, a call failed).
    #[error("the session bus: {0}")]
    Bus(#[from] zbus::Error),
    /// The portal's name has another owner, or the bus refused it.
    #[error("the portal name: {0}")]
    Name(#[from] dbus_kit::name::ClaimError),
}
