//! Downloading one remote cover (effects: the network, the filesystem).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use http_core::HttpError;
use http_reqwest::{Limits, Options, ReqwestHttp};

use super::ART_MAX_BYTES;

/// How long one fetch may take, connecting included.
const FETCH_TIME_LIMIT: Duration = Duration::from_secs(20);

/// How many redirects one fetch follows.
const REDIRECTS_MAX: u8 = 5;

/// The client of every fetch: `http` and `https` (a cover's own address says which), the size
/// and time limits above.
fn client() -> Result<&'static ReqwestHttp, &'static HttpError> {
    static CLIENT: OnceLock<Result<ReqwestHttp, HttpError>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            ReqwestHttp::with_options(&Options {
                plain_http_anywhere: true,
                ..Options::default()
            })
            .map(|http| {
                http.with_limits(Limits {
                    timeout: FETCH_TIME_LIMIT,
                    max_body: usize::try_from(ART_MAX_BYTES).unwrap_or(usize::MAX),
                })
            })
        })
        .as_ref()
}

/// Whether `url` was brought to `cache_path`: fetched within the time and size limits (http and
/// https only, also across redirects), written to a part file, then renamed into place, so a
/// stopped fetch never leaves a half cover that looks cached.
pub(super) async fn fetch(url: &str, cache_path: &Path) -> bool {
    let Some(dir) = cache_path.parent() else {
        return false;
    };
    let Ok(http) = client() else {
        return false;
    };
    let Ok(reply) = http.get(url, REDIRECTS_MAX).await else {
        return false;
    };
    if !reply.status().is_success() {
        return false;
    }
    let _ = tokio::fs::create_dir_all(dir).await;
    let mut part = cache_path.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let written = tokio::fs::write(&part, reply.body()).await.is_ok();
    if written && tokio::fs::rename(&part, cache_path).await.is_ok() {
        return true;
    }
    let _ = tokio::fs::remove_file(&part).await;
    false
}
