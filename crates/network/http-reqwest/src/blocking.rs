//! [`BlockingHttp`]: the same client for a caller with no async runtime. A response's body is
//! read as [`std::io::Read`], so a download is copied to a file as it arrives. Run it off any
//! async runtime (it blocks the thread it is called on).

use http_core::HttpError;
use http_core::http::{HeaderMap, Request, StatusCode};
use reqwest::blocking::Client;
use std::io::Read;
use std::time::Duration;

/// How long a blocking client waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Patience {
    /// Making the connection, handshake included.
    pub connect: Duration,
    /// Any one read of the response, head or body: a connection that brings nothing in this time
    /// is found stalled. There is no cap on the whole download.
    pub read: Duration,
}

/// A client that reads a response as it arrives, over `https` only (also across redirects).
#[derive(Debug, Clone)]
pub struct BlockingHttp {
    client: Client,
}

/// A response whose body has not been read yet.
pub struct Reply {
    /// The status.
    pub status: StatusCode,
    /// The headers.
    pub headers: HeaderMap,
    /// The length the server announced, if it did.
    pub length: Option<u64>,
    /// The body.
    pub body: Box<dyn Read + Send>,
}

impl std::fmt::Debug for Reply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reply")
            .field("status", &self.status)
            .field("length", &self.length)
            .finish_non_exhaustive()
    }
}

impl BlockingHttp {
    /// A client that waits as `patience` says and sends `user_agent`.
    ///
    /// # Errors
    /// [`HttpError::Malformed`] when the client cannot be built.
    pub fn https_only(patience: Patience, user_agent: &str) -> Result<Self, HttpError> {
        // reqwest's rustls takes the process's provider: ring, installed once.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = Client::builder()
            .https_only(true)
            .connect_timeout(patience.connect)
            .read_timeout(patience.read)
            .user_agent(user_agent)
            .build()
            .map_err(HttpError::malformed)?;
        Ok(Self { client })
    }

    /// Sends `request`; the response with its body still to be read. A status that is not a
    /// success is a reply, not an error.
    ///
    /// # Errors
    /// [`HttpError`] when no response head arrived.
    pub fn send(&self, request: Request<Vec<u8>>) -> Result<Reply, HttpError> {
        let https = request.uri().scheme_str() == Some("https");
        let outgoing =
            reqwest::blocking::Request::try_from(request).map_err(|e| crate::fault(https, e))?;
        let response = self
            .client
            .execute(outgoing)
            .map_err(|e| crate::fault(https, e))?;
        Ok(Reply {
            status: response.status(),
            headers: response.headers().clone(),
            length: response.content_length(),
            body: Box::new(response),
        })
    }
}
