//! The one place the network is reached. `Fetch` is the seam; the install and update commands
//! pass `Https`, tests pass `Files` over a directory.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use base_url::WebUrl;
use http_core::http::Method;
use http_reqwest::blocking::{BlockingHttp, Patience};

/// Why bytes did not arrive. Carries the location and the cause.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("could not download {0}")]
pub struct FetchError(pub String);

/// The largest archive the actions service takes, in bytes.
pub const MAX_BYTES: u64 = 1 << 30;

/// The largest small document (the registry list) the actions service takes into memory, in bytes.
pub const MAX_SMALL_BYTES: u64 = 16 << 20;

/// The longest the connection may be silent in the middle of a download, in seconds.
pub const MAX_SECONDS: u64 = 900;

/// The longest the connection may take to be made, in seconds.
pub const CONNECT_SECONDS: u64 = 30;

/// Gets the bytes at a location.
pub trait Fetch {
    /// The bytes at `url`, a small document held in memory (at most [`MAX_SMALL_BYTES`]).
    ///
    /// # Errors
    /// [`FetchError`] when the bytes did not arrive or are too many.
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError>;

    /// Writes the bytes at `url` to the file `to`, which is made or replaced; at most
    /// [`MAX_BYTES`]. Nothing is left at `to` on failure.
    ///
    /// # Errors
    /// [`FetchError`] when the bytes did not arrive or are too many.
    fn download(&self, url: &str, to: &Path) -> Result<(), FetchError>;
}

/// Copies `from` into `to`, at most `max` bytes: more is an error.
fn copy_capped(from: &mut impl Read, to: &mut impl Write, max: u64) -> Result<(), std::io::Error> {
    let copied = std::io::copy(&mut from.take(max.saturating_add(1)), to)?;
    if copied > max {
        return Err(std::io::Error::other(format!("more than {max} bytes")));
    }
    Ok(())
}

/// Fills `to` through `fill`; the file is removed when `fill` fails.
fn into_file(
    url: &str,
    to: &Path,
    fill: impl FnOnce(&mut std::fs::File) -> Result<(), FetchError>,
) -> Result<(), FetchError> {
    let mut out = std::fs::File::create(to).map_err(|e| FetchError(format!("{url}: {e}")))?;
    let done =
        fill(&mut out).and_then(|()| out.flush().map_err(|e| FetchError(format!("{url}: {e}"))));
    if done.is_err() {
        drop(out);
        let _ = std::fs::remove_file(to);
    }
    done
}

/// Downloads over `http-reqwest`'s blocking client: https only (also across redirects), time and
/// size capped.
#[derive(Debug, Clone, Copy)]
pub struct Https;

impl Https {
    /// Streams `url` into `sink`, stopping the transfer past `max` bytes.
    fn stream(url: &str, sink: &mut impl Write, max: u64) -> Result<(), FetchError> {
        let fault = |why: String| FetchError(format!("{url}: {why}"));
        if !url.starts_with("https://") {
            return Err(fault("only https is used".to_owned()));
        }
        let client = BlockingHttp::https_only(
            Patience {
                connect: Duration::from_secs(CONNECT_SECONDS),
                read: Duration::from_secs(MAX_SECONDS),
            },
            "bardo-agents",
        )
        .map_err(|e| fault(e.to_string()))?;
        let web = WebUrl::parse(url).map_err(|e| fault(e.to_string()))?;
        let request = http_core::request(Method::GET, &web)
            .body(Vec::new())
            .map_err(|e| fault(e.to_string()))?;
        let mut reply = client.send(request).map_err(|e| fault(e.to_string()))?;
        if !reply.status.is_success() {
            return Err(fault(format!("the server answered {}", reply.status)));
        }
        copy_capped(&mut reply.body, sink, max).map_err(|e| fault(e.to_string()))
    }
}

impl Fetch for Https {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        let mut bytes = Vec::new();
        Self::stream(url, &mut bytes, MAX_SMALL_BYTES)?;
        Ok(bytes)
    }

    fn download(&self, url: &str, to: &Path) -> Result<(), FetchError> {
        into_file(url, to, |file| Self::stream(url, file, MAX_BYTES))
    }
}

/// Reads `file://NAME` from a directory: for tests and for archives the person already has.
#[derive(Debug, Clone)]
pub struct Files(PathBuf);

impl Files {
    /// Serves the files under `dir`.
    #[must_use]
    pub fn under(dir: &Path) -> Self {
        Self(dir.to_owned())
    }

    /// Opens the file `url` names.
    fn open(&self, url: &str) -> Result<std::fs::File, FetchError> {
        let fault = |why: String| FetchError(format!("{url}: {why}"));
        let name = url
            .strip_prefix("file://")
            .ok_or_else(|| fault("not a file:// location".to_owned()))?;
        let plain = !name.is_empty() && !name.contains("..") && !name.starts_with('/');
        if !plain {
            return Err(fault("not a plain file name".to_owned()));
        }
        std::fs::File::open(self.0.join(name)).map_err(|e| fault(e.to_string()))
    }
}

impl Fetch for Files {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        let mut file = self.open(url)?;
        let mut bytes = Vec::new();
        copy_capped(&mut file, &mut bytes, MAX_SMALL_BYTES)
            .map_err(|e| FetchError(format!("{url}: {e}")))?;
        Ok(bytes)
    }

    fn download(&self, url: &str, to: &Path) -> Result<(), FetchError> {
        let mut from = self.open(url)?;
        into_file(url, to, |file| {
            copy_capped(&mut from, file, MAX_BYTES).map_err(|e| FetchError(format!("{url}: {e}")))
        })
    }
}
