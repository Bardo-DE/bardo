//! Reading and rewriting one request head. Pure: bytes and a plan in, bytes and the body's
//! framing out, or why the request is refused.

use super::reach;
use crate::fault::RelayFault;
use account_core::{RelayAuth, RelayPlan};
use base_url::{Origin, UrlScheme};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

/// How the request's body is framed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framing {
    /// No body.
    None,
    /// This many bytes.
    Length(u64),
    /// Chunked.
    Chunked,
}

/// A rewritten head and the framing of the body that follows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rewritten {
    /// What the server is sent: request line and headers, through the blank line.
    pub head: Vec<u8>,
    /// The body that follows.
    pub framing: Framing,
}

/// The most header lines a request head may carry; one with more is refused.
const MAX_HEADERS: usize = 256;

/// `host[:port]` or `[v6][:port]` as written in a `Host` header or a URI authority: the host in
/// lower case and the port if one is named. User info is refused.
fn authority(text: &str) -> Option<(String, Option<u16>)> {
    if text.contains('@') {
        return None;
    }
    let (host, port) = match text.strip_prefix('[') {
        Some(bracketed) => {
            let (host, after) = bracketed.split_once(']')?;
            (
                host,
                if after.is_empty() {
                    ""
                } else {
                    after.strip_prefix(':')?
                },
            )
        }
        None => text.split_once(':').unwrap_or((text, "")),
    };
    let port = match port {
        "" => None,
        digits => Some(digits.parse::<u16>().ok()?),
    };
    (!host.is_empty()).then(|| (host.to_ascii_lowercase(), port))
}

/// Whether `(host, port)` is `origin`, a missing port being `default`.
fn is_origin(parsed: &(String, Option<u16>), default: u16, origin: &Origin) -> bool {
    parsed.0 == origin.host && parsed.1.unwrap_or(default) == origin.port
}

/// The `Host` value for the origin: the port only when it is not the scheme's default.
fn canonical_host(origin: &Origin) -> String {
    let host = if origin.host.contains(':') {
        format!("[{}]", origin.host)
    } else {
        origin.host.clone()
    };
    if origin.port == origin.scheme.default_port() {
        host
    } else {
        format!("{host}:{}", origin.port)
    }
}

/// The request target as the server gets it (origin-form), checked against `origin`.
fn target_for(target: &str, origin: &Origin) -> Result<String, RelayFault> {
    if target.starts_with('/') || target == "*" {
        return Ok(target.to_owned());
    }
    let (scheme, rest) = target.split_once("://").ok_or(RelayFault::Protocol)?;
    let default = match scheme.to_ascii_lowercase().as_str() {
        "http" => UrlScheme::Http.default_port(),
        "https" => UrlScheme::Https.default_port(),
        _ => return Err(RelayFault::ForeignOrigin),
    };
    let (auth, path) = match rest.find(['/', '?']) {
        Some(at) => rest.split_at(at),
        None => (rest, ""),
    };
    let parsed = authority(auth).ok_or(RelayFault::ForeignOrigin)?;
    match is_origin(&parsed, default, origin) {
        true if path.starts_with('/') => Ok(path.to_owned()),
        true => Ok(format!("/{path}")),
        false => Err(RelayFault::ForeignOrigin),
    }
}

/// The `Authorization` header line the relay adds, or none for a relay that adds no credential.
/// A way of authenticating this relay does not know is `Protocol`, never an anonymous request.
fn credential(auth: &RelayAuth, login: &str) -> Option<String> {
    match auth {
        RelayAuth::Password(password) => Some(format!(
            "Basic {}",
            STANDARD.encode(format!("{login}:{}", password.expose()))
        )),
        RelayAuth::AccessToken(token) => Some(format!("Bearer {}", token.expose())),
        RelayAuth::Anonymous => None,
    }
}

/// How the body is framed, from the values of the `Content-Length` and `Transfer-Encoding`
/// headers the request had.
fn framing_of(lengths: &[&str], codings: &[&str]) -> Result<Framing, RelayFault> {
    match (lengths, codings) {
        ([], []) => Ok(Framing::None),
        ([length], []) => {
            let digits = !length.is_empty() && length.bytes().all(|b| b.is_ascii_digit());
            match digits.then(|| length.parse::<u64>()) {
                Some(Ok(0)) => Ok(Framing::None),
                Some(Ok(n)) => Ok(Framing::Length(n)),
                _ => Err(RelayFault::Protocol),
            }
        }
        ([], [coding]) if coding.eq_ignore_ascii_case("chunked") => Ok(Framing::Chunked),
        // Both, a repeated length or any other coding: the server might read the end of the
        // body somewhere else than the relay does.
        _ => Err(RelayFault::Protocol),
    }
}

/// Reads `head` (request line and headers, through the blank line) and gives the head the
/// server is sent: the target and `Host` checked against the plan's origin, the app's
/// credentials dropped and the relay's added, and the framing of the body.
pub fn rewrite(head: &[u8], plan: &RelayPlan) -> Result<Rewritten, RelayFault> {
    let origin = plan.endpoint.url.origin();
    let Ok(text) = std::str::from_utf8(head) else {
        return Err(RelayFault::Protocol);
    };
    let body = text.strip_suffix("\r\n\r\n").ok_or(RelayFault::Protocol)?;
    if body.bytes().any(|b| b == 0) || body.replace("\r\n", "").contains(['\r', '\n']) {
        return Err(RelayFault::Protocol);
    }
    // httparse reads the request line and the headers, and refuses a method or a name that is
    // not a token, whitespace before the colon, line folding, a control byte in a value and a
    // version other than HTTP/1.0 and 1.1. It stops at the first blank line: anything it did
    // not consume is a second head.
    let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
    let mut request = httparse::Request::new(&mut headers);
    match request.parse(head) {
        Ok(httparse::Status::Complete(used)) if used == head.len() => {}
        _ => return Err(RelayFault::Protocol),
    }
    let (Some(method), Some(target), Some(minor)) = (request.method, request.path, request.version)
    else {
        return Err(RelayFault::Protocol);
    };
    if method.eq_ignore_ascii_case("CONNECT") {
        return Err(RelayFault::Protocol);
    }
    let version = format!("HTTP/1.{minor}");
    let target = target_for(target, &origin)?;

    let mut kept = Vec::new();
    let mut hosts = Vec::new();
    let mut depths = Vec::new();
    let (mut lengths, mut codings) = (Vec::new(), Vec::new());
    for header in request.headers.iter() {
        let Ok(value) = std::str::from_utf8(header.value) else {
            return Err(RelayFault::Protocol);
        };
        let value = value.trim_matches([' ', '\t']);
        let line = if value.is_empty() {
            format!("{}:", header.name)
        } else {
            format!("{}: {value}", header.name)
        };
        match header.name.to_ascii_lowercase().as_str() {
            "authorization" | "proxy-authorization" => {}
            "host" => hosts.push(value),
            "upgrade" => return Err(RelayFault::Protocol),
            "depth" => {
                depths.push(value);
                kept.push(line);
            }
            "content-length" => {
                lengths.push(value);
                kept.push(line);
            }
            "transfer-encoding" => {
                codings.push(value);
                kept.push(line);
            }
            _ => kept.push(line),
        }
    }
    if let Some(host) = hosts.first() {
        let ok = hosts.len() == 1
            && authority(host)
                .is_some_and(|parsed| is_origin(&parsed, origin.scheme.default_port(), &origin));
        match (hosts.len(), ok) {
            (1, true) => {}
            (1, false) => return Err(RelayFault::ForeignOrigin),
            _ => return Err(RelayFault::Protocol),
        }
    }
    let framing = framing_of(&lengths, &codings)?;
    // A repeated Depth could be read one way here and another by the server.
    let depth = match depths.as_slice() {
        [] => None,
        [one] => Some(*one),
        _ => return Err(RelayFault::Protocol),
    };
    reach::check(plan, method, &target, depth)?;

    let mut out = format!(
        "{method} {target} {version}\r\nHost: {}\r\n",
        canonical_host(&origin)
    );
    for line in kept {
        out.push_str(&line);
        out.push_str("\r\n");
    }
    if let Some(value) = credential(&plan.auth, &plan.endpoint.login.0) {
        out.push_str("Authorization: ");
        out.push_str(&value);
        out.push_str("\r\n");
    }
    out.push_str("\r\n");
    Ok(Rewritten {
        head: out.into_bytes(),
        framing,
    })
}
