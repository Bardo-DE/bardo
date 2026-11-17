//! The real downloader: `http-reqwest`'s blocking client over HTTPS (run it off the async
//! runtime).

use super::{Download, Fetch, FetchError, skip};
use base_url::WebUrl;
use http_core::HttpError;
use http_core::http::Method;
use http_reqwest::blocking::{BlockingHttp, Patience};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

/// How long a connection may take to be made.
const CONNECT_WITHIN: Duration = Duration::from_secs(20);
/// The longest one read of a connection is waited for. A connection that brings nothing in this
/// time is found stalled; one that brought data is simply continued by a range request, so a slow
/// link is never given up on, and there is no cap on the whole download.
const READ_WINDOW: Duration = Duration::from_secs(120);

/// Downloads over HTTPS only.
#[derive(Clone, Debug, Default)]
pub struct HttpFetch {
    client: Arc<OnceLock<Result<BlockingHttp, HttpError>>>,
}

impl HttpFetch {
    /// A downloader with a connect time limit and a limit on how long one read is waited for.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn client(&self) -> Result<&BlockingHttp, FetchError> {
        self.client
            .get_or_init(|| {
                BlockingHttp::https_only(
                    Patience {
                        connect: CONNECT_WITHIN,
                        read: READ_WINDOW,
                    },
                    "bardo-updater",
                )
            })
            .as_ref()
            .map_err(failed)
    }
}

fn failed(error: &HttpError) -> FetchError {
    FetchError::Unreachable(error.to_string())
}

impl Fetch for HttpFetch {
    fn open(&self, url: &str) -> Result<Download, FetchError> {
        self.open_from(url, 0)
    }

    fn open_from(&self, url: &str, offset: u64) -> Result<Download, FetchError> {
        let web = WebUrl::parse(url).map_err(|e| FetchError::Unreachable(e.to_string()))?;
        let mut request = http_core::request(Method::GET, &web);
        if offset > 0 {
            request = request.header("Range", format!("bytes={offset}-"));
        }
        let request = request
            .body(Vec::new())
            .map_err(|e| FetchError::Unreachable(e.to_string()))?;
        let reply = self.client()?.send(request).map_err(|e| failed(&e))?;
        if !reply.status.is_success() {
            return Err(FetchError::Refused(reply.status.as_u16()));
        }
        let resumed = reply.status.as_u16() == 206;
        let length = reply.length;
        let mut reader = reply.body;
        if offset > 0 && !resumed {
            // The site ignored the range and sent the file from its start.
            skip(&mut reader, offset)?;
        }
        Ok(Download {
            length,
            body: reader,
        })
    }
}
