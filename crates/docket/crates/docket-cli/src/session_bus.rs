//! The desktop's buses: intentd and the assistant, both on the session bus.

use companion_client::DbusCompanion;
use docket_client::{DbusTransport, Intents, TransportError};

/// The session bus.
#[derive(Debug, Clone, Copy)]
pub struct SessionBus;

impl SessionBus {
    /// Connects to intentd. A connection is made only when a command needs it.
    pub async fn intents(&self) -> Result<Intents<DbusTransport>, TransportError> {
        DbusTransport::connect().await.map(Intents::over)
    }

    /// Connects to the assistant.
    pub async fn companion(&self) -> Result<DbusCompanion, TransportError> {
        DbusCompanion::connect().await
    }
}
