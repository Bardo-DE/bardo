//! What an OAuth token shows about an account, with no IO: the requests are
//! `http::Request<Vec<u8>>` values and the answers are read from `&http::Response<Vec<u8>>`, so
//! the caller sends them with whatever HTTP client it has and recorded answers drive every test.
//!
//! - [`bearer_get`]: the one read, a JSON `GET` with the token as the bearer.
//! - [`google`]: which of an account's services answer, and why one does not.
//! - [`microsoft`]: who the account is on Graph, and which of its services answer.
//!
//! Which path shows which service is data (`issuers.toml`); the readings of the answers are here.

#![warn(missing_docs)]

pub mod google;
pub mod microsoft;

use base_url::WebUrl;
use http::header::{ACCEPT, AUTHORIZATION};
use http::{HeaderValue, Method, Request};

/// Why a read could not be built or its answer not used.
#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    /// The server answered with this status, which the reading does not take as an answer.
    #[error("the server answered with status {0}")]
    Status(u16),
    /// The answer is not what the API sends.
    #[error("the answer is not what the API sends")]
    Unreadable,
    /// The URL is not a web URL.
    #[error("the address is not a web URL")]
    Address(#[source] base_ids::IdError),
    /// The token cannot be sent as a header value.
    #[error("the token is not usable as a header value")]
    Token(#[source] http::header::InvalidHeaderValue),
    /// The request could not be built.
    #[error("the request could not be built")]
    Request(#[source] http::Error),
    /// The answer is not JSON of the expected shape.
    #[error("the answer is not the expected JSON")]
    Json(#[source] serde_json::Error),
}

/// A JSON `GET` of `url` with `token` as the bearer. The header is marked sensitive.
///
/// # Errors
///
/// [`Failure::Unreadable`] when `url` or `token` is not usable in a request, or the request cannot
/// be built.
pub fn bearer_get(url: &str, token: &str) -> Result<Request<Vec<u8>>, ProbeError> {
    let url = WebUrl::parse(url).map_err(ProbeError::Address)?;
    let mut bearer =
        HeaderValue::from_str(&format!("Bearer {token}")).map_err(ProbeError::Token)?;
    bearer.set_sensitive(true);
    Request::builder()
        .method(Method::GET)
        .uri(url.as_str())
        .header(ACCEPT, "application/json")
        .header(AUTHORIZATION, bearer)
        .body(Vec::new())
        .map_err(ProbeError::Request)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_read_carries_the_token_as_a_sensitive_bearer() {
        let request = bearer_get("https://api.example.test/me?x=1", "tok").unwrap();
        assert_eq!(request.method(), "GET");
        assert_eq!(request.uri(), "https://api.example.test/me?x=1");
        assert_eq!(request.headers()[AUTHORIZATION], "Bearer tok");
        assert!(request.headers()[AUTHORIZATION].is_sensitive());
        assert!(!format!("{request:?}").contains("tok"));
    }

    #[test]
    fn a_url_that_is_not_a_web_url_builds_nothing() {
        assert!(matches!(
            bearer_get("imaps://x.example.test", "t"),
            Err(ProbeError::Address(_))
        ));
        assert!(matches!(
            bearer_get("http://api.example.test/", "t"),
            Err(ProbeError::Address(_))
        ));
    }
}
