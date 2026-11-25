//! Serving an app's provider on its own bus name.

use crate::bus::{serve_on, until_closed};
use crate::error::ServeError;
use crate::provider::{ContextSource, IntentProvider, SummonTarget};

/// Serves `org.bardo.IntentProvider1` for `provider` on the app's activatable bus name (the
/// manifest's app) on the session bus, until the connection closes.
///
/// The router calls it only after checking that the name's owner derives to the same `AppId`; every
/// member but `Summon` refuses any caller that is not actiond.
///
/// # Errors
///
/// Returns `ServeError::Bus` when the session bus cannot be reached or refuses the export, and
/// `ServeError::Taken` when another process already owns the app's bus name.
pub async fn serve<P, C, S>(provider: P, context: C, summon: S) -> Result<(), ServeError>
where
    P: IntentProvider + 'static,
    C: ContextSource + 'static,
    S: SummonTarget + 'static,
{
    let connection = action_dbus::BusConnection::session().await?;
    serve_on(&connection, provider, context, summon).await?;
    until_closed(&connection).await;
    Ok(())
}
