//! Serving `org.bardo.Reader1`.

use crate::service::ReaderService;
use action_client::Transport as IntentsTransport;
use action_core::{ReaderAsk, ReaderError};
use action_dbus::callers::{ACTIOND, Identity, require};
use action_dbus::{BusConnection, Details, READER_BUS, READER_PATH};
use action_vocab::Value;
use inference_client::Transport as InferTransport;
use provenance::SessionId;
use std::sync::Arc;
use zbus::fdo;
use zbus::message::Header;

/// Why the daemon stopped serving.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ServeError {
    /// The bus name is taken or the connection failed.
    #[error("bus: {0}")]
    Bus(String),
    /// `readerd-callers.toml` exists and cannot be used.
    #[error("{0}")]
    Callers(String),
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "used as `map_err(bus)`, which hands the error over by value"
)]
fn bus(error: zbus::Error) -> ServeError {
    ServeError::Bus(error.to_string())
}

/// `org.bardo.Reader1` over a service.
struct ReaderObject<P: InferTransport, I: IntentsTransport> {
    service: Arc<ReaderService<P, I>>,
    callers: Arc<Identity>,
}

impl<P: InferTransport, I: IntentsTransport> std::fmt::Debug for ReaderObject<P, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReaderObject")
    }
}

#[zbus::interface(name = "org.bardo.Reader1")]
impl<P: InferTransport + 'static, I: IntentsTransport + 'static> ReaderObject<P, I> {
    async fn extract(
        &self,
        session: String,
        ask: String,
        options: Details,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<String> {
        let _ = options;
        let sender = header.sender().map(zbus::names::UniqueName::as_str);
        if let Err(refused) = require(&self.callers, sender, &[ACTIOND]).await {
            tracing::warn!(%refused, "refused a caller that is not actiond");
            return Err(refused.into());
        }
        let session = SessionId::parse(&session).map_err(|error| {
            tracing::warn!(%error, "refused a request whose session is not an id");
            fdo::Error::InvalidArgs(format!("not a session id: {error}"))
        })?;
        let ask: ReaderAsk = serde_json::from_str(&ask).map_err(|error| {
            tracing::warn!("refused a request that is not a reader ask");
            fdo::Error::InvalidArgs(format!("not a reader ask: {error}"))
        })?;
        let answer: Result<Value, ReaderError> = self.service.extract_in(&session, ask).await;
        // Which way it failed, never what was read or said.
        if let Err(error) = &answer {
            let reason = match error {
                ReaderError::OutOfSchema(_) => "out_of_schema",
                ReaderError::Unparseable => "unparseable",
                ReaderError::Refused => "refused",
                ReaderError::ModelUnavailable => "model_unavailable",
            };
            tracing::warn!(reason, "the reader gave no answer");
        }
        serde_json::to_string(&answer).map_err(|e| fdo::Error::Failed(e.to_string()))
    }
}

/// Exports `org.bardo.Reader1` at its path on `connection` and claims the name.
///
/// Only the process that `callers` names as actiond (the `agent` role of the app
/// `org.bardo.Intents`) may ask. Returns once the name is ours; the connection keeps serving until
/// it closes.
///
/// # Errors
///
/// Returns [`ServeError`] when the daemon could not start serving or stopped serving.
pub async fn serve_on<P, I>(
    connection: &BusConnection,
    service: Arc<ReaderService<P, I>>,
    callers: Arc<Identity>,
) -> Result<(), ServeError>
where
    P: InferTransport + 'static,
    I: IntentsTransport + 'static,
{
    let object = ReaderObject { service, callers };
    connection
        .object_server()
        .at(READER_PATH, object)
        .await
        .map_err(bus)?;
    dbus_kit::ready::serving(connection).await.map_err(bus)?;
    dbus_kit::name::claim(connection, READER_BUS)
        .await
        .map_err(|error| ServeError::Bus(error.to_string()))
}

/// Claims `org.bardo.Reader1` on the session bus and serves `Extract(session, ask)` until the
/// connection closes.
///
/// Only actiond may call it: the process behind the caller is named by `readerd-callers.toml` (the
/// system's under `/etc/bardo`, the person's over it) before the ask is read.
///
/// # Errors
///
/// Returns [`ServeError`] when the daemon could not start serving or stopped serving.
pub async fn serve<P, I>(
    service: Arc<ReaderService<P, I>>,
    env: &impl Fn(&str) -> Option<String>,
) -> Result<(), ServeError>
where
    P: InferTransport + 'static,
    I: IntentsTransport + 'static,
{
    use futures_util::StreamExt;
    let connection = BusConnection::session().await.map_err(bus)?;
    let table = crate::callers::read(env).map_err(|e| ServeError::Callers(e.to_string()))?;
    let who = Arc::new(Identity::on_system(connection.clone(), table));
    serve_on(&connection, service, who).await?;
    let mut messages = zbus::MessageStream::from(&connection);
    while messages.next().await.is_some() {}
    Ok(())
}
