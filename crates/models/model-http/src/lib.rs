//! HTTP to a model endpoint: a local engine on a Unix socket, a loopback port, or a
//! cloud host over TLS, possibly through the egress proxy.
//!
//! This crate is pure: the decoders of server-sent events and of NDJSON, the request an exchange
//! comes to, and the endpoint's rules about keys are values and functions. The connection is the
//! `http-core` stream an [`HttpClient`] is given (`model-http-reqwest` builds the real one), so
//! the crates that need only these types reach no HTTP stack and no runtime. A codec
//! (`model-wire`) builds an `Exchange` and reads the response from a `BodySink`, which sees the
//! `ResponseHead` before the first byte of the body.
//!
//! ```
//! use model_http::{
//!     AuthHeader, HostName, HttpEndpoint, HttpTarget, Port, Proxy, Timeouts, UrlPath, WaitMs,
//! };
//!
//! // A local engine on a loopback port, no credentials.
//! let endpoint = HttpEndpoint {
//!     target: HttpTarget::Tcp { host: HostName("127.0.0.1".into()), port: Port(8000) },
//!     proxy: Proxy::Direct,
//!     base: UrlPath("/v1".into()),
//!     auth: AuthHeader::None,
//!     headers: vec![],
//!     timeouts: Timeouts { connect: WaitMs(2_000), first_byte: WaitMs(60_000), idle: WaitMs(30_000) },
//! };
//! assert_eq!(endpoint.base, UrlPath("/v1".into()));
//! ```
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

mod auth;
mod client;
mod exchange;
mod head;
mod ndjson;
mod request;
mod sse;
mod target;
mod transport;

pub use auth::{AuthHeader, ExtraHeader, HeaderName};
pub use client::{
    BodySink, ContentType, DerCertificate, HttpClient, HttpError, HttpStatus, JsonBody, RawBody,
    TlsRoots,
};
pub use exchange::{
    Exchange, Framing, RouteRoot, Timeouts, Transport, Upload, UploadTransport, Verb,
};
pub use head::{BodyKind, RequestId, RequestIdError, ResponseHead, WaitMs, WaitSeconds};
pub use ndjson::{LineError, NdjsonDecoder};
pub use sse::{EventName, SseDecoder, SseError, SseEvent};
pub use target::{HostName, HttpEndpoint, HttpTarget, Port, Proxy, UrlPath};
