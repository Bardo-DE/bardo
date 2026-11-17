//! `Transport for HttpClient`: the exchange written as one HTTP request, sent through the
//! client's [`HttpStream`], and the response delivered to the sink with its head first.
//!
//! A `Tls` target is TLS over TCP; the stream that was given decides whether it can speak it (a
//! stream that cannot answers `HttpError::Tls`). The egress proxy is not built yet: a `Via`
//! proxy answers `HttpError::Connect` for every target, `Tls` included, without sending anything.
//!
//! The stages keep their timeouts. The response head must arrive within the connect and
//! first-byte timeouts together, because the stream hands over the response only once the head is
//! read; the idle timeout runs between chunks of the body that made progress. The sink's
//! `deadline` bounds the whole exchange, connect included. A request is one connection: dropping
//! the future drops it, which aborts generation in the engine.

use std::ops::ControlFlow;
use std::time::Duration;

use http_core::http::{HeaderMap, StatusCode};
use http_core::{HttpStream, Waits};

use crate::request::{Parts, prepare};
use crate::{
    BodySink, Exchange, HttpClient, HttpError, HttpStatus, Proxy, ResponseHead, Timeouts,
    Transport, Upload, UploadTransport, WaitMs,
};

impl Transport for HttpClient {
    fn exchange<K: BodySink>(
        &self,
        ex: &Exchange,
        sink: &mut K,
    ) -> impl Future<Output = Result<HttpStatus, HttpError>> + Send {
        send(self, Parts::of_exchange(ex), sink)
    }
}

impl UploadTransport for HttpClient {
    fn upload<K: BodySink>(
        &self,
        up: &Upload,
        sink: &mut K,
    ) -> impl Future<Output = Result<HttpStatus, HttpError>> + Send {
        send(self, Parts::of_upload(up), sink)
    }
}

/// `wait` as a duration.
fn millis(wait: WaitMs) -> Duration {
    Duration::from_millis(u64::from(wait.0))
}

/// The waits of `timeouts` and the sink's deadline.
fn waits_of(timeouts: &Timeouts, total: Option<WaitMs>) -> Waits {
    Waits {
        head: millis(timeouts.connect).saturating_add(millis(timeouts.first_byte)),
        idle: millis(timeouts.idle),
        total: total.map(millis),
    }
}

/// What a failure of the request means to a codec.
fn refused(error: &http_core::HttpError) -> HttpError {
    use http_core::HttpError as Core;
    match error {
        Core::TimedOut => HttpError::Timeout,
        Core::Tls(_) => HttpError::Tls,
        Core::Unreachable(_) | Core::NotGranted => HttpError::Connect,
        Core::Malformed(_) | Core::TooLarge => HttpError::Broken,
    }
}

/// Hands the sink the head and the body of the response as the stream delivers them.
struct Relay<'a, K> {
    sink: &'a mut K,
}

impl<K: BodySink> http_core::BodySink for Relay<'_, K> {
    fn head(&mut self, status: StatusCode, headers: &HeaderMap) -> ControlFlow<()> {
        self.sink.head(&ResponseHead::of(status, headers))
    }

    fn chunk(&mut self, bytes: &[u8]) -> ControlFlow<()> {
        self.sink.chunk(bytes)
    }

    fn progress(&self) -> http_core::Progress {
        self.sink.progress()
    }
}

/// Writes the request, connects, and reads the response into `sink`.
async fn send<K: BodySink>(
    client: &HttpClient,
    parts: Parts<'_>,
    sink: &mut K,
) -> Result<HttpStatus, HttpError> {
    let endpoint = client.endpoint();
    let request = prepare(endpoint, parts)?;
    if matches!(endpoint.proxy, Proxy::Via(_)) {
        return Err(HttpError::Connect);
    }
    let waits = waits_of(&endpoint.timeouts, sink.deadline());
    let status = client
        .stream()
        .stream(request, waits, &mut Relay { sink })
        .await
        .map_err(|error| {
            tracing::warn!(error = %error, "a request to the engine failed");
            refused(&error)
        })?;
    outcome(HttpStatus(status.as_u16()))
}

/// A success is its status; anything else is `Rejected`, its head and body already in the sink.
fn outcome(status: HttpStatus) -> Result<HttpStatus, HttpError> {
    if (200..300).contains(&status.0) {
        Ok(status)
    } else {
        Err(HttpError::Rejected)
    }
}
