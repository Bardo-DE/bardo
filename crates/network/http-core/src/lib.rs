//! The one HTTP seam (design/31 D12): the [`Http`] trait every family, discovery step, OAuth
//! exchange, sync replica and model call sends through, over the [`http`] crate's request and response. A family never names a
//! client; the daemon hands it `http_reqwest::ReqwestHttp`, a test hands it a fake server's.
//! Bodies are whole byte vectors (`Request<Vec<u8>>`, `Response<Vec<u8>>`): what the accounts service sends and reads
//! over HTTP (sign-in exchanges, PROPFIND replies, model lists) is small, and a transfer of file
//! content belongs to the replica that streams it. The `http` crate is re-exported, so a caller
//! names the same version the accounts service does.
//!
//! A request's URL is an accounts service [`WebUrl`](base_url::WebUrl) (`https`, or `http` to this
//! computer); [`request`] turns it into the request's URI.
//!
//! The clients are other crates: `http-reqwest` for a direct connection and
//! `http-relay` over an accountd relay.
//!
//! Requests and responses are plain values:
//!
//! ```
//! use base_time::UnixSeconds;
//! use base_url::WebUrl;
//! use http_core::http::{Response, StatusCode};
//! use http_core::{HeaderText, request};
//!
//! let url = WebUrl::parse("https://cloud.example.org/remote.php/dav/").expect("a web url");
//! let sent = request(http_core::webdav::propfind(), &url)
//!     .header("Depth", "1")
//!     .body(b"<propfind/>".to_vec())
//!     .expect("a request");
//! assert_eq!(sent.method().as_str(), "PROPFIND");
//! assert_eq!(sent.headers().len(), 1);
//!
//! let answer: Response<Vec<u8>> = Response::builder()
//!     .status(StatusCode::TOO_MANY_REQUESTS)
//!     .header("Retry-After", "30")
//!     .body(Vec::new())
//!     .expect("a response");
//! assert!(!answer.status().is_success());
//! assert_eq!(answer.retry_after_seconds(UnixSeconds(0)), Some(30));
//! ```

mod error;
mod message;
mod seam;
mod shared;
mod stream;
pub mod webdav;

pub use error::{Cause, HttpError};
pub use http;
pub use message::{HeaderText, SensitiveHeader, request, uri};
pub use seam::Http;
pub use shared::SharedHttp;
pub use stream::{BodySink, HttpStream, Progress, SharedStream, Waits};
