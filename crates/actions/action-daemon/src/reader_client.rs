//! The quarantined reader, over `org.bardo.Reader1`.

use action_core::{Reader, ReaderAsk, ReaderError};
use action_dbus::{BusConnection, Details, ReaderProxy};
use action_vocab::Value;
use provenance::{Quarantined, SessionId};

/// The quarantined reader, over `org.bardo.Reader1`.
#[derive(Debug, Clone)]
pub struct ReaderClient {
    connection: BusConnection,
}

impl ReaderClient {
    /// Calls readerd over `connection`.
    #[must_use]
    pub fn new(connection: BusConnection) -> Self {
        Self { connection }
    }

    /// `Reader1.Extract(session, ask)`: readerd resolves the ask's handles in `session` through
    /// `Session.Resolve` (the inputs stay in actiond's handle table) and answers a value that
    /// fits the schema, as the JSON of `Result<Value, ReaderError>`. A reader that is not on the
    /// bus is `ModelUnavailable`: never a value, never a panic.
    ///
    /// # Errors
    ///
    /// Returns a [`ReaderError`] when readerd cannot be reached, refuses, or answers something that
    /// is not valid or does not fit the schema.
    pub async fn extract_in(
        &self,
        session: &SessionId,
        ask: &ReaderAsk,
    ) -> Result<Value, ReaderError> {
        let ask = serde_json::to_string(ask).map_err(|error| {
            tracing::warn!(error = %error, "the reader's ask could not be written as JSON");
            ReaderError::Unparseable
        })?;
        let proxy = ReaderProxy::new(&self.connection).await.map_err(|error| {
            tracing::warn!(error = %error, "readerd is not on the bus");
            ReaderError::ModelUnavailable
        })?;
        let answer = proxy
            .extract(session.as_str(), &ask, &Details::new())
            .await
            .map_err(|error| {
                tracing::warn!(error = %error, "readerd's Extract call failed");
                ReaderError::ModelUnavailable
            })?;
        decode(&answer)?
    }
}

/// The JSON of `Result<Value, ReaderError>` readerd answered.
#[expect(
    clippy::map_err_ignore,
    reason = "the decode error quotes the reply, which holds the model's text and the person's data"
)]
fn decode(answer: &str) -> Result<Result<Value, ReaderError>, ReaderError> {
    serde_json::from_str(answer).map_err(|_| ReaderError::Unparseable)
}

impl Reader for ReaderClient {
    /// The seam hands over the quarantined text too, but `Reader1.Extract` names the session
    /// instead: readerd resolves the handles itself, so the text never crosses the bus.
    async fn extract(
        &self,
        session: &SessionId,
        ask: ReaderAsk,
        _inputs: Vec<Quarantined<String>>,
    ) -> Result<Value, ReaderError> {
        self.extract_in(session, &ask).await
    }
}
