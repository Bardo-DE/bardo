//! The request a transport sends: the URL, the headers and the body of one exchange or upload
//! against an endpoint. Nothing is connected here; a request that cannot be written is never sent.

use http_core::http::header::{
    ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderName as WireName, HeaderValue,
};
use http_core::http::{Method, Request, Uri};

use crate::{
    AuthHeader, Exchange, Framing, HttpEndpoint, HttpError, HttpTarget, RouteRoot, Upload, Verb,
};

/// What a request is, whichever seam asked for it.
pub(crate) struct Parts<'a> {
    pub method: Method,
    pub root: RouteRoot,
    pub path: &'a str,
    pub framing: Framing,
    pub content_type: Option<&'a str>,
    pub body: Vec<u8>,
}

impl<'a> Parts<'a> {
    pub(crate) fn of_exchange(ex: &'a Exchange) -> Self {
        Self {
            method: match ex.verb {
                Verb::Get => Method::GET,
                Verb::PostJson => Method::POST,
            },
            root: ex.root,
            path: ex.path.0.as_str(),
            framing: ex.framing,
            content_type: ex.body.as_ref().map(|_| "application/json"),
            body: ex
                .body
                .as_ref()
                .map_or_else(Vec::new, |b| b.0.clone().into_bytes()),
        }
    }

    pub(crate) fn of_upload(up: &'a Upload) -> Self {
        Self {
            method: Method::POST,
            root: up.root,
            path: up.path.0.as_str(),
            framing: up.framing,
            content_type: Some(up.body.content_type.0.as_str()),
            body: up.body.bytes.clone(),
        }
    }
}

/// Whether a key may go to a plain `Tcp` host on this machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Loopback {
    Refused,
    Allowed,
}

/// A key goes over TLS or a Unix socket; over plain `Tcp` only to a loopback host, and only when
/// the caller opted in.
fn allow_key(target: &HttpTarget, loopback: Loopback) -> Result<(), HttpError> {
    match target {
        HttpTarget::Tcp { host, .. } if loopback == Loopback::Allowed && is_loopback(&host.0) => {
            Ok(())
        }
        HttpTarget::Tcp { .. } => Err(HttpError::Insecure),
        _ => Ok(()),
    }
}

fn is_loopback(host: &str) -> bool {
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    bare.eq_ignore_ascii_case("localhost")
        || bare
            .parse::<core::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

/// The URI of `path` on `target`. A Unix socket has no host; the client reaches the socket and
/// ignores the name, and `localhost` is what the `Host` header says. A host that is not a name or
/// an address is `HttpError::Tls` for a `Tls` target (found before anything is connected) and
/// `Connect` for the others.
fn url_of(target: &HttpTarget, path: &str) -> Result<Uri, HttpError> {
    let (scheme, authority, wrong) = match target {
        HttpTarget::Tcp { host, port } => ("http", authority(&host.0, port.0), HttpError::Connect),
        HttpTarget::Tls { host, port } => ("https", authority(&host.0, port.0), HttpError::Tls),
        HttpTarget::Unix(_) => ("http", "localhost".to_owned(), HttpError::Connect),
    };
    format!("{scheme}://{authority}{path}")
        .parse::<Uri>()
        .map_err(|error| {
            tracing::warn!(%error, "the engine's address is not a URL");
            wrong
        })
}

/// `host:port`, with a bare IPv6 address in brackets.
fn authority(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

/// The request of `parts` against `endpoint`. A header value that cannot be
/// written (a newline in a secret) is a request that is never sent.
pub(crate) fn prepare(
    endpoint: &HttpEndpoint,
    parts: Parts<'_>,
) -> Result<Request<Vec<u8>>, HttpError> {
    let path = match parts.root {
        RouteRoot::Base => format!("{}{}", endpoint.base.0, parts.path),
        RouteRoot::Server => parts.path.to_owned(),
    };
    let accept = match parts.framing {
        Framing::Sse => "text/event-stream",
        Framing::Ndjson => "application/x-ndjson",
        Framing::Whole => "application/json",
    };
    let mut builder = Request::builder()
        .method(parts.method)
        .uri(url_of(&endpoint.target, &path)?);
    let mut headers = vec![(ACCEPT, HeaderValue::from_static(accept))];
    if let Some(content_type) = parts.content_type {
        let value = HeaderValue::from_str(content_type).map_err(|error| {
            tracing::warn!(error = %error, "the content type is not a header value");
            HttpError::Connect
        })?;
        headers.push((CONTENT_TYPE, value));
    }
    let secret = |name: WireName, value: &str| {
        HeaderValue::from_str(value)
            .map(|mut value| {
                value.set_sensitive(true);
                (name, value)
            })
            .map_err(|error| {
                tracing::warn!(error = %error, "a header value cannot be written");
                HttpError::Connect
            })
    };
    let mut keyed = Vec::new();
    match &endpoint.auth {
        AuthHeader::None => {}
        AuthHeader::Bearer(token) => {
            allow_key(&endpoint.target, Loopback::Refused)?;
            keyed.push(secret(
                AUTHORIZATION,
                &format!("Bearer {}", token.expose()),
            )?);
        }
        AuthHeader::LoopbackBearer(token) => {
            allow_key(&endpoint.target, Loopback::Allowed)?;
            keyed.push(secret(
                AUTHORIZATION,
                &format!("Bearer {}", token.expose()),
            )?);
        }
        AuthHeader::Header { name, value } => {
            let name = WireName::from_bytes(name.0.as_bytes()).map_err(|error| {
                tracing::warn!(error = %error, "the key header name is not a header name");
                HttpError::Connect
            })?;
            keyed.push(secret(name, value.expose())?);
        }
    }
    for extra in &endpoint.headers {
        let name = WireName::from_bytes(extra.name.0.as_bytes()).map_err(|error| {
            tracing::warn!(error = %error, "an extra header name is not a header name");
            HttpError::Connect
        })?;
        keyed.push(secret(name, extra.value.expose())?);
    }
    for (name, value) in headers.into_iter().chain(keyed) {
        builder = builder.header(name, value);
    }
    builder.body(parts.body).map_err(|error| {
        tracing::warn!(error = %error, "the request cannot be built");
        HttpError::Connect
    })
}
