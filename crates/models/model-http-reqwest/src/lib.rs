//! The connection of a model endpoint: [`client`] and [`client_with_roots`] give a
//! [`HttpClient`](model_http::HttpClient) whose requests go through `http-reqwest`, built as the
//! endpoint says.
//!
//! The client is built on its first exchange and shared by the clones of the `HttpClient`. It
//! follows no redirect, uses no proxy from the environment, keeps no connection after its
//! exchange (so dropping the exchange drops the connection, which aborts generation in the
//! engine), and its connect timeout covers the TCP connect and the TLS handshake. A `Unix`
//! target is the socket; a `Tls` target is rustls on the ring provider, the platform's roots or
//! only those given. The chain, the validity dates and the name are always checked; there is no
//! switch to turn that off.

use http_core::http::{Request, StatusCode};
use http_core::{BodySink, HttpError, HttpStream, SharedStream, Waits};
use http_reqwest::{Options, ReqwestHttp};
use model_http::{HttpClient, HttpEndpoint, HttpTarget, TlsRoots};
use std::sync::OnceLock;
use std::time::Duration;

/// A client for `endpoint` that trusts the platform's roots for a `Tls` target.
#[must_use]
pub fn client(endpoint: HttpEndpoint) -> HttpClient {
    client_with_roots(endpoint, TlsRoots::Platform)
}

/// A client for `endpoint` that trusts `roots` for a `Tls` target.
#[must_use]
pub fn client_with_roots(endpoint: HttpEndpoint, roots: TlsRoots) -> HttpClient {
    let options = options_of(&endpoint, &roots);
    HttpClient::new(
        endpoint,
        roots,
        SharedStream::new(Lazy {
            options,
            built: OnceLock::new(),
        }),
    )
}

/// How `http-reqwest` is built for an endpoint and its roots.
fn options_of(endpoint: &HttpEndpoint, roots: &TlsRoots) -> Options {
    let (roots, only) = match roots {
        TlsRoots::Platform => (Vec::new(), false),
        TlsRoots::Only(only) => (only.iter().map(|cert| cert.0.clone()).collect(), true),
    };
    Options {
        roots,
        only_these_roots: only,
        unix_socket: match &endpoint.target {
            HttpTarget::Unix(path) => Some(path.clone()),
            _ => None,
        },
        connect_timeout: Some(Duration::from_millis(u64::from(
            endpoint.timeouts.connect.0,
        ))),
        one_connection_per_request: true,
        // A key never goes over plain `http` to another computer: `model-http` refuses to write
        // such a request, and a keyless engine on a LAN host is the caller's choice.
        plain_http_anywhere: true,
    }
}

/// The reqwest client, built on the first request; a client that could not be built answers
/// every request with why.
struct Lazy {
    options: Options,
    built: OnceLock<Result<ReqwestHttp, HttpError>>,
}

impl HttpStream for Lazy {
    async fn stream(
        &self,
        request: Request<Vec<u8>>,
        waits: Waits,
        sink: &mut dyn BodySink,
    ) -> Result<StatusCode, HttpError> {
        match self
            .built
            .get_or_init(|| ReqwestHttp::with_options(&self.options))
        {
            Ok(http) => http.stream(request, waits, sink).await,
            Err(error) => Err(error.clone()),
        }
    }
}
