//! The one request line the proxy understands, read with every size bounded. `CONNECT host:port`
//! (https) and `METHOD http://host[:port]/... HTTP/1.x` (plain http); anything else is refused.

use super::host::{EgressHostError, Host};
use crate::forward::Duplex;
use std::io;
use std::time::{Duration, Instant};

/// The longest line (the request line, or one header) in bytes.
pub const MAX_LINE: usize = 8 * 1024;
/// The most bytes of request line and headers together.
pub const MAX_HEAD: usize = 32 * 1024;
/// How long the whole head may take to arrive.
pub const HEAD_DEADLINE: Duration = Duration::from_secs(10);

/// Why a request was not understood. Every one is answered with a plain 400, except a stream
/// that closed before it said anything, which is just dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BadRequestError {
    /// A line or the whole head is longer than allowed.
    #[error("the request is too long")]
    TooLong,
    /// The head did not arrive in time.
    #[error("the request took too long")]
    Timeout,
    /// The stream closed with nothing sent.
    #[error("the stream closed")]
    Closed,
    /// The stream closed in the middle of the head.
    #[error("the request ended early")]
    Truncated,
    /// Reading failed.
    #[error("reading the request failed")]
    Io(io::ErrorKind),
    /// Not `METHOD target HTTP/1.x`, or not text.
    #[error("the request line is malformed")]
    Malformed,
    /// The target names no host.
    #[error("the request names no host")]
    NoHost,
    /// The port is not a number from 1 to 65535.
    #[error("the port is not valid")]
    BadPort,
    /// The host is not valid.
    #[error("the host is not valid: {0}")]
    BadHost(EgressHostError),
    /// Not a proxy request: a path without a host, an `https://` URI, or user information.
    #[error("this is not a proxy request for http or CONNECT")]
    NotProxy,
}

/// Where a request wants to go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The host.
    pub host: Host,
    /// The port.
    pub port: u16,
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

/// What the request asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// `CONNECT`: a tunnel, answered with `200` before any bytes pass.
    Tunnel(Target),
    /// A plain http request, forwarded unchanged.
    Relay(Target),
}

impl Request {
    /// Where it goes.
    #[must_use]
    pub fn target(&self) -> &Target {
        match self {
            Request::Tunnel(t) | Request::Relay(t) => t,
        }
    }
}

/// The bytes read so far, and where the head (request line and headers) ends in them. Anything
/// after `end` was sent early and belongs to the upstream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    /// Everything read.
    pub bytes: Vec<u8>,
    /// The index just past the blank line.
    pub end: usize,
}

fn head_end(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|at| at + 4)
}

fn first_line_too_long(bytes: &[u8]) -> bool {
    match bytes.windows(2).position(|w| w == b"\r\n") {
        Some(at) => at > MAX_LINE,
        None => bytes.len() > MAX_LINE,
    }
}

/// Reads until the blank line that ends the headers, within `wait` and `MAX_HEAD`.
///
/// # Errors
/// Returns the `BadRequestError` for a head that is too long, late, cut off, empty, or unreadable.
pub fn read_head<S: Duplex>(stream: &mut S, wait: Duration) -> Result<Head, BadRequestError> {
    let deadline = Instant::now() + wait;
    let mut bytes: Vec<u8> = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        if let Some(end) = head_end(&bytes) {
            return if end > MAX_HEAD {
                Err(BadRequestError::TooLong)
            } else {
                Ok(Head { bytes, end })
            };
        }
        if bytes.len() >= MAX_HEAD || first_line_too_long(&bytes) {
            return Err(BadRequestError::TooLong);
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(BadRequestError::Timeout);
        }
        stream
            .limit_reads(Some(left))
            .map_err(|e| BadRequestError::Io(e.kind()))?;
        match stream.read(&mut chunk) {
            Ok(0) if bytes.is_empty() => return Err(BadRequestError::Closed),
            Ok(0) => return Err(BadRequestError::Truncated),
            Ok(n) => bytes.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Err(BadRequestError::Timeout);
            }
            Err(e) => return Err(BadRequestError::Io(e.kind())),
        }
    }
}

fn port_of(text: &str) -> Result<u16, BadRequestError> {
    let digits = !text.is_empty() && text.len() <= 5 && text.bytes().all(|b| b.is_ascii_digit());
    match text.parse::<u16>() {
        Ok(port) if digits && port != 0 => Ok(port),
        _ => Err(BadRequestError::BadPort),
    }
}

fn host_of(text: &str) -> Result<Host, BadRequestError> {
    Host::parse(text).map_err(|fault| match fault {
        EgressHostError::Empty => BadRequestError::NoHost,
        other => BadRequestError::BadHost(other),
    })
}

/// `host`, `host:port`, `[v6]` or `[v6]:port`; `default` is the port when there is none.
fn authority(text: &str, default: Option<u16>) -> Result<Target, BadRequestError> {
    let (host, port) = match text.strip_prefix('[') {
        Some(inner) => {
            let (addr, rest) = inner.split_once(']').ok_or(BadRequestError::Malformed)?;
            let port = match rest {
                "" => None,
                _ => Some(rest.strip_prefix(':').ok_or(BadRequestError::Malformed)?),
            };
            (format!("[{addr}]"), port)
        }
        None => match text.rsplit_once(':') {
            // An IPv6 address without brackets would be cut at its last colon.
            Some((host, _)) if host.contains(':') => return Err(BadRequestError::Malformed),
            Some((host, port)) => (host.to_owned(), Some(port)),
            None => (text.to_owned(), None),
        },
    };
    let port = match (port, default) {
        (Some(text), _) => port_of(text)?,
        (None, Some(port)) => port,
        (None, None) => return Err(BadRequestError::BadPort),
    };
    Ok(Target {
        host: host_of(&host)?,
        port,
    })
}

/// The absolute `http://` target's authority, or why it is not one.
fn http_authority(target: &str) -> Result<&str, BadRequestError> {
    let scheme = target.get(..7).ok_or(BadRequestError::NotProxy)?;
    if !scheme.eq_ignore_ascii_case("http://") {
        return Err(BadRequestError::NotProxy);
    }
    let rest = &target[7..];
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let auth = &rest[..end];
    if auth.contains('@') {
        return Err(BadRequestError::NotProxy);
    }
    Ok(auth)
}

/// The request in `head`: its first line says what it wants, and no line is longer than
/// [`MAX_LINE`]. The headers are not read, only bounded.
///
/// # Errors
/// Returns the `BadRequestError` for a head with an over-long line, a malformed request line, a target
/// without a valid host, or a port that is not 1 to 65535.
pub fn parse_head(head: &[u8]) -> Result<Request, BadRequestError> {
    if head
        .split(|b| *b == b'\n')
        .any(|line| line.len() > MAX_LINE)
    {
        return Err(BadRequestError::TooLong);
    }
    let line_end = head
        .windows(2)
        .position(|w| w == b"\r\n")
        .ok_or(BadRequestError::Malformed)?;
    let Ok(line) = std::str::from_utf8(&head[..line_end]) else {
        return Err(BadRequestError::Malformed);
    };
    let words: Vec<&str> = line.split(' ').collect();
    let &[method, target, version] = words.as_slice() else {
        return Err(BadRequestError::Malformed);
    };
    let method_ok =
        !method.is_empty() && method.len() <= 16 && method.bytes().all(|b| b.is_ascii_uppercase());
    if !method_ok || !matches!(version, "HTTP/1.0" | "HTTP/1.1") || target.is_empty() {
        return Err(BadRequestError::Malformed);
    }
    if method == "CONNECT" {
        return authority(target, None).map(Request::Tunnel);
    }
    authority(http_authority(target)?, Some(80)).map(Request::Relay)
}
