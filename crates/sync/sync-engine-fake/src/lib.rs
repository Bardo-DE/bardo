//! Test tooling, never a normal dependency: a dataset held in memory and the scheduler settings
//! the test rigs poll with, for the engine's tests (and for tests of the driver and the daemon
//! that need an engine running).
//!
//! `MemoryDataset`: local ids are paths; a test edits files through `put` and `remove`.

use base_ids::Bytes;
use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};
use sync_core::{Blob, ConflictRule, ItemPath, LocalId, Scanned};
use sync_engine::dataset::{Dataset, DatasetError, DatasetId, fingerprint};
use sync_engine::scheduler::{MeteredPolicy, Settings};

/// Files by path.
#[derive(Debug)]
pub struct MemoryDataset {
    id: DatasetId,
    rule: ConflictRule,
    files: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl MemoryDataset {
    /// An empty dataset named `id` (a slug) settling conflicts by `rule`.
    ///
    /// # Panics
    ///
    /// Panics when `id` is not a dataset slug.
    #[must_use]
    pub fn new(id: &str, rule: ConflictRule) -> Self {
        Self {
            id: DatasetId::parse(id).expect("a dataset slug"),
            rule,
            files: Mutex::default(),
        }
    }

    fn files(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, Vec<u8>>> {
        self.files.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Writes a file as the local user would.
    pub fn put(&self, path: &str, bytes: &[u8]) {
        self.files().insert(path.to_owned(), bytes.to_vec());
    }

    /// Deletes a file as the local user would.
    pub fn remove(&self, path: &str) {
        self.files().remove(path);
    }

    /// The file at `path`.
    pub fn get(&self, path: &str) -> Option<Vec<u8>> {
        self.files().get(path).cloned()
    }

    /// Every file, by path.
    pub fn snapshot(&self) -> BTreeMap<String, Vec<u8>> {
        self.files().clone()
    }
}

fn scanned(path: &str, bytes: &[u8]) -> Scanned {
    Scanned {
        local: LocalId(path.to_owned()),
        path: ItemPath(path.to_owned()),
        size: Bytes(bytes.len() as u64),
        hash: fingerprint(bytes),
    }
}

impl Dataset for MemoryDataset {
    fn id(&self) -> DatasetId {
        self.id.clone()
    }

    fn conflict_rule(&self) -> ConflictRule {
        self.rule
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn scan(&self) -> Result<Vec<Scanned>, DatasetError> {
        Ok(self
            .files()
            .iter()
            .map(|(path, bytes)| scanned(path, bytes))
            .collect())
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn read(&self, item: &LocalId) -> Result<Blob, DatasetError> {
        self.get(&item.0)
            .map(Blob)
            .ok_or_else(|| DatasetError(format!("{} is not held", item.0)))
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn store(
        &self,
        at: Option<&LocalId>,
        path: &ItemPath,
        content: Blob,
    ) -> Result<Scanned, DatasetError> {
        let mut files = self.files();
        if let Some(LocalId(old)) = at
            && *old != path.0
        {
            files.remove(old);
        }
        let stored = scanned(&path.0, &content.0);
        files.insert(path.0.clone(), content.0);
        Ok(stored)
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn discard(&self, item: &LocalId) -> Result<(), DatasetError> {
        self.files().remove(&item.0);
        Ok(())
    }
}

/// What every test rig polls with: every second, never batched, and the scheduler's clock ten
/// times the real one, so a poll takes a tenth of a second.
#[must_use]
pub fn quick_settings() -> Settings {
    Settings {
        poll_base: 1,
        poll_max: 1,
        push_window: 0,
        batch_window: 0,
        metered: MeteredPolicy::Pause,
        time_scale: 10,
    }
}
