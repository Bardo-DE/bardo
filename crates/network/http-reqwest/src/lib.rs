//! The one direct-connection HTTP client: [`ReqwestHttp`], reqwest behind the
//! [`Http`](http_core::Http) and [`HttpStream`](http_core::HttpStream) seams of `http-core`.
//!
//! TLS is rustls on `ring` with the platform's trust roots (plus any the caller adds, which is
//! how a test trusts its scratch CA); certificate checks are never off. Plain `http` is for this
//! computer only unless the caller opts in to any host (an engine on another computer): a request
//! to any other host over `http` is refused before it is dialled. Redirects are not followed (a
//! caller that wants one reads `Location`), the whole exchange has a timeout, and a response
//! longer than the cap is `TooLarge`.
//!
//! [`blocking::BlockingHttp`] is the same for a caller with no runtime (a download that reads
//! its body as `io::Read`).
//!
//! This is the only crate in the workspace that names reqwest.

pub mod blocking;

use base_url::{Origin, UrlScheme};
use http_core::http::header::LOCATION;
use http_core::http::{Method, Request, Response, StatusCode, Uri};
use http_core::{BodySink, HttpError, HttpStream, Progress, Waits};
use reqwest::redirect::Policy;
use reqwest::{Certificate, Client};
use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;

/// How long a request may take and how much of a response is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The whole exchange: connecting, the handshake, the request and the whole response.
    pub timeout: Duration,
    /// The most a response body may hold, in bytes.
    pub max_body: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_body: 8 * 1024 * 1024,
        }
    }
}

/// How a client is built, beyond the defaults (the platform's roots, connections kept and
/// reused, the proxy of the environment, plain `http` to this computer only).
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// DER certificates trusted beside the platform's roots (a private CA), or instead of them
    /// when [`only_these_roots`](Self::only_these_roots) is set.
    pub roots: Vec<Vec<u8>>,
    /// Trust only [`roots`](Self::roots) and nothing else: the seam for a test's scratch CA. The
    /// chain, the validity dates and the host name are checked either way.
    pub only_these_roots: bool,
    /// Reach every request's server through this Unix socket (the URL's host is only the `Host`
    /// header).
    pub unix_socket: Option<PathBuf>,
    /// One connection per request and none kept, and no proxy from the environment: dropping a
    /// request's future drops its connection, which stops generation in a model engine.
    pub one_connection_per_request: bool,
    /// How long making the connection may take, handshake included (`None`: only the waits of
    /// the request apply).
    pub connect_timeout: Option<Duration>,
    /// Allow plain `http` to any host, not only this computer. The caller keeps credentials off
    /// such a connection itself.
    pub plain_http_anywhere: bool,
}

/// reqwest behind the [`Http`](http_core::Http) and [`HttpStream`](http_core::HttpStream) seams.
///
/// It gives connection reuse, a timeout, a cap on the response size, and TLS through rustls with
/// the system's trust roots.
#[derive(Debug, Clone)]
pub struct ReqwestHttp {
    client: Client,
    limits: Limits,
    plain_anywhere: bool,
}

impl Default for ReqwestHttp {
    fn default() -> Self {
        Self::new()
    }
}

impl ReqwestHttp {
    /// A client trusting the platform's roots.
    #[must_use]
    pub fn new() -> Self {
        Self::with_extra_roots(&[])
    }

    /// A client trusting the platform's roots and these DER certificates (a private CA).
    /// A certificate that is not one is skipped.
    #[must_use]
    pub fn with_extra_roots(extra: &[Vec<u8>]) -> Self {
        Self::with_options(&Options {
            roots: extra.to_vec(),
            ..Options::default()
        })
        .unwrap_or_else(|_| unreachable!("a client with rustls and no proxy builds"))
    }

    /// A client built as `options` say.
    ///
    /// # Errors
    /// [`HttpError::Tls`] when a certificate of a client that trusts only its own roots is not
    /// one; [`HttpError::Malformed`] when the client cannot be built.
    pub fn with_options(options: &Options) -> Result<Self, HttpError> {
        // reqwest's rustls takes the process's provider: ring, installed once (a second install,
        // or one another crate made first, is refused and harmless).
        let _ = rustls::crypto::ring::default_provider().install_default();
        let mut builder = Client::builder().redirect(Policy::none());
        builder = if options.one_connection_per_request {
            builder.no_proxy().pool_max_idle_per_host(0)
        } else {
            builder.pool_idle_timeout(Duration::from_secs(30))
        };
        if let Some(wait) = options.connect_timeout {
            builder = builder.connect_timeout(wait);
        }
        if let Some(path) = &options.unix_socket {
            builder = builder.unix_socket(path.as_path());
        }
        builder = if options.only_these_roots {
            let certs = options
                .roots
                .iter()
                .map(|der| Certificate::from_der(der))
                .collect::<Result<Vec<_>, _>>()
                .map_err(HttpError::tls)?;
            builder.tls_certs_only(certs)
        } else {
            options
                .roots
                .iter()
                .filter_map(|der| Certificate::from_der(der).ok())
                .fold(builder, reqwest::ClientBuilder::add_root_certificate)
        };
        Ok(Self {
            client: builder.build().map_err(HttpError::malformed)?,
            limits: Limits::default(),
            plain_anywhere: options.plain_http_anywhere,
        })
    }

    /// The same client with other limits.
    #[must_use]
    pub fn with_limits(self, limits: Limits) -> Self {
        Self { limits, ..self }
    }

    /// The request as reqwest sends it, with the exchange's timeout.
    fn outgoing(
        &self,
        request: Request<Vec<u8>>,
        timeout: Option<Duration>,
    ) -> Result<reqwest::Request, HttpError> {
        if !self.plain_anywhere {
            in_the_clear_only_here(request.uri())?;
        }
        let https = request.uri().scheme_str() == Some("https");
        let mut outgoing = reqwest::Request::try_from(request).map_err(|e| fault(https, e))?;
        *outgoing.timeout_mut() = timeout;
        Ok(outgoing)
    }

    async fn exchange(&self, request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, HttpError> {
        let https = request.uri().scheme_str() == Some("https");
        let outgoing = self.outgoing(request, Some(self.limits.timeout))?;
        let mut response = self
            .client
            .execute(outgoing)
            .await
            .map_err(|e| fault(https, e))?;
        let mut bytes = Vec::new();
        while let Some(data) = response.chunk().await.map_err(|e| fault(https, e))? {
            if bytes.len() + data.len() > self.limits.max_body {
                return Err(HttpError::TooLarge);
            }
            bytes.extend_from_slice(&data);
        }
        let mut answer = Response::new(bytes);
        *answer.status_mut() = response.status();
        *answer.version_mut() = response.version();
        *answer.headers_mut() = response.headers().clone();
        Ok(answer)
    }

    /// The response head, then the body chunk by chunk to `sink`. The idle timer restarts only
    /// when a chunk made progress, so keep-alive comments and empty events do not hold a request
    /// open.
    async fn pour(
        &self,
        request: Request<Vec<u8>>,
        waits: Waits,
        sink: &mut dyn BodySink,
    ) -> Result<StatusCode, HttpError> {
        let https = request.uri().scheme_str() == Some("https");
        let outgoing = self.outgoing(request, None)?;
        let mut response = tokio::time::timeout(waits.head, self.client.execute(outgoing))
            .await
            .map_or(Err(HttpError::TimedOut), |sent| {
                sent.map_err(|e| fault(https, e))
            })?;
        let status = response.status();
        let mut flow = sink.head(status, response.headers());
        let mut timer = Box::pin(tokio::time::sleep(waits.idle));
        while flow.is_continue() {
            let next = tokio::select! {
                () = &mut timer => return Err(HttpError::TimedOut),
                next = response.chunk() => next,
            };
            match next.map_err(|e| fault(https, e))? {
                None => break,
                Some(data) => {
                    flow = sink.chunk(&data);
                    if sink.progress() == Progress::Made {
                        timer = Box::pin(tokio::time::sleep(waits.idle));
                    }
                }
            }
        }
        Ok(status)
    }
}

impl ReqwestHttp {
    /// `GET url`, following up to `redirects` redirects (each by the `Location` header, absolute
    /// or an absolute path). Every hop is held to the client's own rules (plain `http` only to
    /// this computer unless the client was built to allow any host).
    ///
    /// # Errors
    /// [`HttpError`] as for [`Http::send`](http_core::Http::send); [`HttpError::Malformed`] for a
    /// URL or a `Location` that is not one, or more redirects than `redirects`.
    pub async fn get(&self, url: &str, redirects: u8) -> Result<Response<Vec<u8>>, HttpError> {
        let mut uri: Uri = url.parse().map_err(HttpError::malformed)?;
        for _ in 0..=redirects {
            let request = Request::builder()
                .method(Method::GET)
                .uri(uri.clone())
                .body(Vec::new())?;
            let response = http_core::Http::send(self, request).await?;
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok());
            match location {
                Some(location) if response.status().is_redirection() => {
                    uri = resolve(&uri, location)?;
                }
                _ => return Ok(response),
            }
        }
        Err(HttpError::Malformed(None))
    }
}

/// `location` as the URI of the next request: itself when it names a scheme, else on `base`'s
/// scheme and host.
fn resolve(base: &Uri, location: &str) -> Result<Uri, HttpError> {
    let target: Uri = location.parse().map_err(HttpError::malformed)?;
    if target.scheme().is_some() {
        return Ok(target);
    }
    let mut parts = target.into_parts();
    parts.scheme = base.scheme().cloned();
    parts.authority = base.authority().cloned();
    Uri::from_parts(parts).map_err(HttpError::malformed)
}

impl http_core::Http for ReqwestHttp {
    async fn send(&self, request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, HttpError> {
        tokio::time::timeout(self.limits.timeout, self.exchange(request))
            .await
            .unwrap_or(Err(HttpError::TimedOut))
    }
}

impl HttpStream for ReqwestHttp {
    async fn stream(
        &self,
        request: Request<Vec<u8>>,
        waits: Waits,
        sink: &mut dyn BodySink,
    ) -> Result<StatusCode, HttpError> {
        let run = self.pour(request, waits, sink);
        match waits.total {
            Some(total) => tokio::time::timeout(total, run)
                .await
                .unwrap_or(Err(HttpError::TimedOut)),
            None => run.await,
        }
    }
}

/// Plain `http` goes to this computer only: a request to any other host over `http` is `Tls`
/// and is never dialled, and a scheme that is not HTTP is `Malformed`.
fn in_the_clear_only_here(uri: &Uri) -> Result<(), HttpError> {
    match uri.scheme_str() {
        Some("https") => Ok(()),
        Some("http") => {
            let host = uri.host().unwrap_or_default();
            let origin = Origin {
                scheme: UrlScheme::Http,
                host: host.trim_matches(['[', ']']).to_ascii_lowercase(),
                port: uri.port_u16().unwrap_or(UrlScheme::Http.default_port()),
            };
            if origin.is_loopback() {
                Ok(())
            } else {
                Err(HttpError::Tls(None))
            }
        }
        _ => Err(HttpError::Malformed(None)),
    }
}

/// What a reqwest error comes to: a timeout, a certificate or handshake refusal, no route, or
/// garbage. On an `https` request a peer that hangs up while connecting is a failed handshake.
pub(crate) fn fault(https: bool, error: reqwest::Error) -> HttpError {
    let tls = is_tls(&error) || (https && error.is_connect() && hung_up(&error));
    match (error.is_timeout(), tls, error.is_connect()) {
        (true, _, _) => HttpError::TimedOut,
        (false, true, _) => HttpError::tls(error),
        (false, false, true) => HttpError::unreachable(error),
        (false, false, false) => HttpError::malformed(error),
    }
}

/// Whether a rustls error is anywhere in the error's chain (tokio-rustls wraps it in `io::Error`s,
/// which do not pass what they wrap on as a `source`).
fn is_tls(error: &(dyn Error + 'static)) -> bool {
    let mut current = Some(error);
    while let Some(err) = current {
        if err.is::<rustls::Error>() || wraps_tls(err) {
            return true;
        }
        current = err.source();
    }
    false
}

fn wraps_tls(error: &(dyn Error + 'static)) -> bool {
    error
        .downcast_ref::<std::io::Error>()
        .and_then(|io| io.get_ref())
        .is_some_and(|inner| inner.is::<rustls::Error>() || wraps_tls(inner))
}

/// Whether an `io::Error` anywhere in the chain is an unexpected end of stream: the peer closed
/// the connection in the middle of the handshake.
fn hung_up(error: &(dyn Error + 'static)) -> bool {
    let mut current = Some(error);
    while let Some(err) = current {
        if err
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::UnexpectedEof)
        {
            return true;
        }
        current = err.source();
    }
    false
}

/// Sleeps on tokio's timer: what a sign-in that polls waits with (the daemon hands it in).
#[derive(Debug, Clone, Copy, Default)]
pub struct TokioSleep;

impl http_core::Sleep for TokioSleep {
    async fn sleep(&self, how_long: Duration) {
        tokio::time::sleep(how_long).await;
    }
}
