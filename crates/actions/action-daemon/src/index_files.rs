//! What apps pushed to the index, kept across logins: `bardo/intents/index/<AppName>.json`
//! under the person's data directory, one file per app, `{"version":1,"entries":[...]}`. Each
//! write replaces the file whole (`base_fs`'s atomic write, mode 0600), so a crash leaves the old file or the new one. The entries are not encrypted: they
//! are as exposed as the app's own store.

use account_core::AppName;
use action_core::IndexEntry;
use action_router::{Router, Seams};
use base_fs::atomic::AtomicWrite;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The version of the file this build writes and reads.
const VERSION: u32 = 1;

/// Why an index file could not be written or read.
#[derive(Debug, thiserror::Error)]
pub enum IndexFileFault {
    /// The disk refused.
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// The text is not the JSON of a file of this version.
    #[error("not an index file: {0}")]
    Malformed(String),
}

/// The file as it is on disk.
#[derive(Debug, Serialize, Deserialize)]
struct IndexFile {
    version: u32,
    entries: Vec<IndexEntry>,
}

/// The directory of index files.
#[derive(Debug, Clone)]
pub struct IndexStore {
    dir: PathBuf,
    /// Held while a file is written, so two writes of one app land in the order they were made.
    writing: Arc<Mutex<()>>,
}

impl IndexStore {
    /// The store under a data directory: `bardo/intents/index`.
    #[must_use]
    pub fn under(data: &Path) -> Self {
        Self {
            dir: crate::manifests::intents_dir(data).join("index"),
            writing: Arc::default(),
        }
    }

    fn file_of(&self, app: &AppName) -> PathBuf {
        self.dir.join(format!("{app}.json"))
    }

    /// Replaces the file of `app` with `entries`.
    ///
    /// # Errors
    ///
    /// Returns an [`IndexFileFault`] when the entries cannot be encoded or the disk refuses the
    /// file.
    pub fn write(&self, app: &AppName, entries: Vec<IndexEntry>) -> Result<(), IndexFileFault> {
        let bytes = serde_json::to_vec(&IndexFile {
            version: VERSION,
            entries,
        })
        .map_err(|e| IndexFileFault::Malformed(e.to_string()))?;
        Ok(AtomicWrite::PRIVATE.write(&self.file_of(app), &bytes)?)
    }

    /// Reads the file at `path`.
    fn read(path: &Path) -> Result<Vec<IndexEntry>, IndexFileFault> {
        let file: IndexFile = serde_json::from_slice(&std::fs::read(path)?)
            .map_err(|e| IndexFileFault::Malformed(e.to_string()))?;
        match file.version {
            VERSION => Ok(file.entries),
            other => Err(IndexFileFault::Malformed(format!("version {other}"))),
        }
    }

    /// Writes the entries `app` has pushed to `router` now. Done outside the router's lock.
    pub fn save<S: Seams>(&self, router: &Router<S>, app: &AppName) {
        let _held = self
            .writing
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Err(why) = self.write(app, router.pushed_entries(app)) {
            eprintln!("intentd: the index of {app} was not kept: {why}");
        }
    }

    /// Brings back the entries of every file here whose app has a manifest in `router`; a file of
    /// an app with none is deleted; one that cannot be read is said so and left.
    pub fn restore<S: Seams>(&self, router: &Router<S>) {
        let Ok(files) = std::fs::read_dir(&self.dir) else {
            return;
        };
        for path in files.filter_map(Result::ok).map(|f| f.path()) {
            let app = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".json"))
                .and_then(|n| AppName::parse(n).ok());
            let Some(app) = app else {
                continue;
            };
            if !router.has_manifest(&app) {
                let _ = std::fs::remove_file(&path);
                continue;
            }
            match Self::read(&path) {
                Ok(entries) => router.restore_index(&app, entries),
                Err(why) => eprintln!("intentd: {} not read: {why}", path.display()),
            }
        }
    }
}
