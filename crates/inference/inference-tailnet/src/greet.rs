//! Reading a lending computer's hello over a connection already made (and checked, see
//! [`crate::Dialer`]).

use crate::hello::{HELLO_PATH, Hello};
use crate::refusal::Refusal;
use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper::client::conn::http1;
use hyper::{Method, Request, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

/// The most of a hello read.
const MOST: usize = 64 * 1024;

/// Why a hello was not read.
#[derive(Debug, thiserror::Error)]
pub enum GreetError {
    /// The computer turned this one away: with the reason it gave (its code in the answer's
    /// body), or `None` for a code this build does not know.
    #[error("the computer refused: {}", .0.map_or("a reason this build does not know", |r| r.slug()))]
    Refused(Option<Refusal>),
    /// Something answered that does not lend models.
    #[error("the answer is not a lending computer's")]
    NotLending,
    /// The connection broke.
    #[error("the connection broke")]
    Broken(#[source] hyper::Error),
    /// The request for the hello could not be built.
    #[error("the request for the hello could not be built")]
    Request(#[source] hyper::http::Error),
}

/// What a refusal body says: the refusal it carries (`Refused(None)` for a code not known here),
/// or `NotLending` when it is not a refusal body at all.
fn refusal_error(body: &[u8]) -> GreetError {
    let code = serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.get("error")?.get("code")?.as_str().map(str::to_owned));
    code.map_or(GreetError::NotLending, |code| {
        GreetError::Refused(Refusal::from_slug(&code))
    })
}

/// Asks the computer on `stream` for its hello. `host` is what the request names as the host.
///
/// # Errors
///
/// Returns [`GreetError`] when the hello could not be read.
pub async fn greet(stream: TcpStream, host: &str) -> Result<Hello, GreetError> {
    let (mut sender, connection) = http1::handshake::<_, Empty<Bytes>>(TokioIo::new(stream))
        .await
        .map_err(GreetError::Broken)?;
    let driver = tokio::spawn(async move {
        let _ = connection.await;
    });
    let request = Request::builder()
        .method(Method::GET)
        .uri(HELLO_PATH)
        .header(hyper::header::HOST, host)
        .body(Empty::<Bytes>::new())
        .map_err(GreetError::Request)?;
    let answer = async {
        let response = sender
            .send_request(request)
            .await
            .map_err(GreetError::Broken)?;
        let status = response.status();
        let mut body = response.into_body();
        let mut bytes = Vec::new();
        while let Some(frame) = body.frame().await {
            let frame = frame.map_err(GreetError::Broken)?;
            if let Some(data) = frame.data_ref() {
                if bytes.len() + data.len() > MOST {
                    return Err(GreetError::NotLending);
                }
                bytes.extend_from_slice(data);
            }
        }
        Ok((status, bytes))
    }
    .await;
    driver.abort();
    let (status, bytes) = answer?;
    match status {
        StatusCode::OK => Hello::parse(&bytes).ok_or(GreetError::NotLending),
        _ => Err(refusal_error(&bytes)),
    }
}
