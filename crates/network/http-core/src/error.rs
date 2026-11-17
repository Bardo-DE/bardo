//! Why a request produced no response.

use std::error::Error;
use std::sync::Arc;

/// The lower-layer error behind an [`HttpError`], shared so the error stays `Clone`.
pub type Cause = Arc<dyn Error + Send + Sync + 'static>;

/// Why a request produced no response. A response with an error status is a response, not this.
///
/// Two faults are equal when they are the same kind of failure; the cause is not compared.
#[derive(Debug, Clone, thiserror::Error)]
pub enum HttpError {
    /// The server could not be reached (no route, refused, a name that does not resolve).
    #[error("unreachable")]
    Unreachable(#[source] Option<Cause>),
    /// The TLS handshake failed or the certificate was refused.
    #[error("tls failure")]
    Tls(#[source] Option<Cause>),
    /// No answer in time.
    #[error("timed out")]
    TimedOut,
    /// The response was longer than this client reads.
    #[error("response too large")]
    TooLarge,
    /// The server did not speak HTTP, or the request could not be built.
    #[error("malformed response")]
    Malformed(#[source] Option<Cause>),
    /// No connection was opened because the account has not given the access this client needs
    /// yet: the relay host that opens connections answered `NeedsConsent` (a whole drive, asked
    /// when first used). Trying again does not help until the person gives it.
    #[error("access not given yet")]
    NotGranted,
}

impl HttpError {
    /// The server could not be reached, because of `source`.
    pub fn unreachable(source: impl Error + Send + Sync + 'static) -> Self {
        Self::Unreachable(Some(Arc::new(source)))
    }

    /// The TLS handshake failed, because of `source`.
    pub fn tls(source: impl Error + Send + Sync + 'static) -> Self {
        Self::Tls(Some(Arc::new(source)))
    }

    /// The exchange was malformed, because of `source`.
    pub fn malformed(source: impl Error + Send + Sync + 'static) -> Self {
        Self::Malformed(Some(Arc::new(source)))
    }
}

impl PartialEq for HttpError {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Eq for HttpError {}

impl From<http::Error> for HttpError {
    /// A request the `http` crate would not build (a header value with a line break, a URL it
    /// does not take) is never sent: `Malformed`, as a reply that is not HTTP is.
    fn from(error: http::Error) -> Self {
        Self::malformed(error)
    }
}
