//! The streaming half of the seam: a response whose body is pushed to a [`BodySink`] as it
//! arrives, for a body too long to hold (a download) or one a caller reads piece by piece (a
//! model's server-sent events or NDJSON lines, whose frames the caller splits).

use crate::error::HttpError;
use http::{HeaderMap, Request, StatusCode};
use std::ops::ControlFlow;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

/// Whether the last chunk carried anything of the reply. A client restarts its idle timer only on
/// `Made`, so a stream of keep-alive comments or empty events cannot hold a request open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Progress {
    /// The chunk carried something.
    Made,
    /// The chunk carried nothing the caller counts.
    Idle,
}

/// Receives a response as it arrives: its head once, then each piece of the body in turn.
///
/// Each answers whether to go on reading (`Continue`) or to stop because the rest is not wanted
/// (`Break`, and the connection is dropped).
pub trait BodySink: Send {
    /// The status and headers, before any of the body. `Break` reads no body.
    fn head(&mut self, status: StatusCode, headers: &HeaderMap) -> ControlFlow<()>;

    /// The next piece of the body.
    fn chunk(&mut self, bytes: &[u8]) -> ControlFlow<()>;

    /// Whether the chunk just delivered made progress. The default counts every chunk.
    fn progress(&self) -> Progress {
        Progress::Made
    }
}

/// How long a streamed request may wait at each stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Waits {
    /// Connecting, the handshake, sending the request and reading the response head.
    pub head: Duration,
    /// Between two chunks that made [`Progress`].
    pub idle: Duration,
    /// The whole exchange, connecting included (`None`: only the stages above apply).
    pub total: Option<Duration>,
}

/// Sends one request and pushes the response to a [`BodySink`]. Redirects are not followed, so a
/// caller reads `Location` from the head and asks again. The same errors as [`Http`](crate::Http):
/// a wait that ran out is [`HttpError::TimedOut`].
pub trait HttpStream: Send + Sync {
    /// Sends `request` and gives the response to `sink`; the status when the body ended or the
    /// sink stopped it.
    fn stream(
        &self,
        request: Request<Vec<u8>>,
        waits: Waits,
        sink: &mut dyn BodySink,
    ) -> impl Future<Output = Result<StatusCode, HttpError>> + Send;
}

type Poured<'a> = Pin<Box<dyn Future<Output = Result<StatusCode, HttpError>> + Send + 'a>>;

trait DynStream: Send + Sync {
    fn stream_boxed<'a>(
        &'a self,
        request: Request<Vec<u8>>,
        waits: Waits,
        sink: &'a mut dyn BodySink,
    ) -> Poured<'a>;
}

impl<H: HttpStream> DynStream for H {
    fn stream_boxed<'a>(
        &'a self,
        request: Request<Vec<u8>>,
        waits: Waits,
        sink: &'a mut dyn BodySink,
    ) -> Poured<'a> {
        Box::pin(self.stream(request, waits, sink))
    }
}

/// Any [`HttpStream`], shared and cloneable, so a holder needs no type parameter. The cost is
/// one boxed future per request, which is nothing next to the request.
#[derive(Clone)]
pub struct SharedStream(Arc<dyn DynStream>);

impl SharedStream {
    /// Shares `stream`.
    pub fn new(stream: impl HttpStream + 'static) -> Self {
        Self(Arc::new(stream))
    }
}

impl std::fmt::Debug for SharedStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SharedStream")
    }
}

impl HttpStream for SharedStream {
    fn stream(
        &self,
        request: Request<Vec<u8>>,
        waits: Waits,
        sink: &mut dyn BodySink,
    ) -> impl Future<Output = Result<StatusCode, HttpError>> + Send {
        self.0.stream_boxed(request, waits, sink)
    }
}
