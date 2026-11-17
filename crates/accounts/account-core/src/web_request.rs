//! An endpoint's URL as an HTTP request: the one place the account vocabulary meets the HTTP
//! seam (`http-core`), which does not know what an endpoint is.

use crate::EndpointUrl;
use base_url::{UrlScheme, WebUrl};
use http_core::http::Method;
use http_core::http::request::Builder;
use http_core::{HttpError, request};

/// A request to an endpoint's URL. A scheme that is not HTTP is `Malformed`, and plain `http`
/// to another computer is `Tls`: no credential is sent in the clear.
///
/// # Errors
/// [`HttpError::Malformed`] when the scheme is not HTTP; [`HttpError::Tls`] for plain `http` to
/// another computer.
pub fn request_to(method: Method, url: &EndpointUrl) -> Result<Builder, HttpError> {
    web_url(url).map(|web| request(method, &web))
}

/// An endpoint's URL as a web URL. A scheme that is not HTTP is `Malformed`, and plain `http`
/// to another computer is `Tls`: no credential is sent in the clear.
///
/// # Errors
/// [`HttpError::Malformed`] when the scheme is not HTTP; [`HttpError::Tls`] for plain `http` to
/// another computer.
pub fn web_url(url: &EndpointUrl) -> Result<WebUrl, HttpError> {
    WebUrl::try_from(url).map_err(|error| match url.origin().scheme {
        UrlScheme::Http => HttpError::tls(error),
        _ => HttpError::malformed(error),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_to_an_endpoint_refuses_what_is_not_web() {
        let cases = [
            ("https://cloud.example.org/dav/", Ok(())),
            ("http://127.0.0.1:8080/dav/", Ok(())),
            ("http://cloud.example.org/dav/", Err(HttpError::Tls(None))),
            ("imaps://mail.example.org", Err(HttpError::Malformed(None))),
        ];
        for (text, want) in cases {
            let url = EndpointUrl::parse(text).expect(text);
            let got = request_to(Method::GET, &url).map(|_| ());
            assert_eq!(got, want, "{text}");
        }
        let url = EndpointUrl::parse("https://cloud.example.org/dav/").expect("url");
        let sent = request_to(Method::GET, &url)
            .expect("builder")
            .body(Vec::<u8>::new())
            .expect("request");
        assert_eq!(sent.uri().to_string(), url.as_str());
    }
}
