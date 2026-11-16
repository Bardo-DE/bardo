//! `rig.json`: what `sync-rig-servers` writes once the issuer and the Graph fake are listening,
//! and what a scenario reads to learn addresses and secrets. The shapes of the entries are
//! `account-rig`'s, so a scenario reads an issuer or an HTTP fake the same way in either rig.
//! Written whole under a temporary name and renamed, so a reader that finds the file finds all of
//! it.

use account_rig::rigfile::{FILE, HttpAt, OauthAt};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Everything the rig started.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RigFile {
    /// The scratch directory.
    pub dir: PathBuf,
    /// The rig's process id: what a scenario stops with SIGTERM.
    pub pid: u32,
    /// The control endpoint's URL (`http://127.0.0.1:<port>`).
    pub control: String,
    /// Every planted secret, for a scan.
    pub secrets: Vec<String>,
    /// The OAuth issuer.
    pub oauth: OauthAt,
    /// The Graph fake (it accepts the issuer's access tokens).
    pub graph: HttpAt,
}

impl RigFile {
    /// The path of the file under `dir`.
    #[must_use]
    pub fn path(dir: &Path) -> PathBuf {
        dir.join(FILE)
    }

    /// Writes the file under `dir` whole.
    ///
    /// # Errors
    ///
    /// The I/O error of encoding the file or writing it under `dir`.
    pub fn write(&self, dir: &Path) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        base_fs::atomic::AtomicWrite::SHARED.write(&Self::path(dir), text.as_bytes())
    }

    /// Reads the file under `dir`.
    ///
    /// # Errors
    ///
    /// The I/O error of reading the file under `dir`, or of its text not being a rig file.
    pub fn read(dir: &Path) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(Self::path(dir))?;
        serde_json::from_str(&text).map_err(std::io::Error::other)
    }
}
