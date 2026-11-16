//! The desktop's buses: intentd and the assistant, both on the session bus.

use action_client::{DbusTransport, Intents, TransportError};
use companion_client::DbusCompanion;

/// The session bus.
#[derive(Debug, Clone, Copy)]
pub struct SessionBus;

impl SessionBus {
    /// Connects to intentd. A connection is made only when a command needs it.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError`] when the request got no reply.
    pub async fn intents(&self) -> Result<Intents<DbusTransport>, TransportError> {
        DbusTransport::connect().await.map(Intents::over)
    }

    /// Connects to the assistant.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError`] when the request got no reply.
    pub async fn companion(&self) -> Result<DbusCompanion, TransportError> {
        DbusCompanion::connect().await
    }
}
