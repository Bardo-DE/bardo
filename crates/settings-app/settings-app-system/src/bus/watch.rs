//! A tick each time a system service changes hands or says anything: the page reads it again.

use super::io::{connect, owner_changes};
use super::system::system_connection;
use settings_app_link::bus::{BusSource, Daemon};
use settings_app_link::{Deadline, peer};
use settings_app_link::{Ticker, Ticks};
use settings_app_model::Absence;
use std::pin::Pin;
use ui_peer::within;
use zbus::export::futures_core::Stream;
use zbus::fdo::DBusProxy;
use zbus::message::Type as MessageType;
use zbus::{Connection, MatchRule, Message, MessageStream};

/// Tells `out` each time `stream` yields, until the stream ends or nobody listens.
pub async fn relay(mut stream: impl Stream<Item: Send> + Unpin + Send + 'static, out: Ticker) {
    while std::future::poll_fn(|cx| Pin::new(&mut stream).poll_next(cx))
        .await
        .is_some()
    {
        if out.tick().is_err() {
            return;
        }
    }
}

/// Whether a signal from the service is worth reading the service again for.
pub type Matters = fn(&Message) -> bool;

/// Every signal matters.
#[must_use]
pub fn every(_: &Message) -> bool {
    true
}

/// Tells `out` each time `stream` yields a message that matters, until the stream ends or nobody
/// listens.
async fn relay_matching(mut stream: MessageStream, matters: Matters, out: Ticker) {
    while let Some(message) = next(&mut stream).await {
        let Ok(message) = message else { continue };
        if matters(&message) && out.tick().is_err() {
            return;
        }
    }
}

/// The ticks for the service `name` changing hands and for each signal it sends that `matters`.
///
/// They are subscribed before anyone reads, so no change falls between the reading and the
/// listening. The relays end with the ticks.
///
/// # Errors
///
/// `Absence::Failed` when the system bus cannot be reached or the subscription is not taken in
/// time.
pub async fn watch_service(
    bus: &BusSource,
    name: &'static str,
    matters: Matters,
) -> Result<Ticks, Absence> {
    let connection = system_connection(bus).await?;
    watch_on(&connection, name, matters).await
}

/// [`watch_service`] on a connection already made (the session bus, for the shell's names).
///
/// # Errors
///
/// `Absence::Failed` when the subscription is not taken in time.
pub async fn watch_on(
    connection: &Connection,
    name: &'static str,
    matters: Matters,
) -> Result<Ticks, Absence> {
    let subscribed = async {
        let owners = DBusProxy::new(connection)
            .await?
            .receive_name_owner_changed_with_args(&[(0, name)])
            .await?;
        let rule = MatchRule::builder()
            .msg_type(MessageType::Signal)
            .sender(name)?
            .build();
        let signals = MessageStream::for_match_rule(rule, connection, None).await?;
        Ok::<_, zbus::Error>((owners, signals))
    };
    let Ok(Ok((owners, signals))) = within(Deadline::READ, peer::DESKTOP, subscribed).await else {
        return Err(Absence::Failed);
    };
    let (ticker, ticks) = Ticks::channel();
    Ok(ticks
        .with_relay(tokio::spawn(relay(owners, ticker.clone())))
        .with_relay(tokio::spawn(relay_matching(signals, matters, ticker))))
}

/// The ticks for inferd's name changing hands, subscribed before anyone reads, so no change falls
/// between the reading and the listening. The relay ends with the ticks.
///
/// # Errors
///
/// `Absence::NoBus` when there is no bus to look on, `Absence::Failed` when the bus cannot be
/// reached or does not take the subscription in time.
pub async fn inferd_owners(bus: &BusSource) -> Result<Ticks, Absence> {
    let connection = connect(bus).await?;
    let owners = owner_changes(&connection, &Daemon::inferd()).await?;
    let (ticker, ticks) = Ticks::channel();
    Ok(ticks.with_relay(tokio::spawn(relay(owners, ticker))))
}

/// The next item of one `Unpin` stream; `None` once it has ended.
pub async fn next<S>(stream: &mut S) -> Option<S::Item>
where
    S: Stream + Unpin,
{
    std::future::poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx)).await
}
