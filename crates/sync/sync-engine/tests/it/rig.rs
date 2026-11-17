//! The shared test rig: a world of one replica, one dataset, one clock and a journal file that
//! engines can be opened on again, as a restarted process would.

use account_capability::{Access, Delta, HashKind, StorageCap, StorageScope};
use base_ids::Bytes;
use base_time::UnixSeconds;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use sync_core::{
    BaseVersion, Blob, ByteRange, Change, ChangePage, ConflictRule, Cursor, ItemPath,
    MemoryReplica, PutError, PutItem, PutTarget, Quota, RemoteId, RemoteVersion, Replica,
    ReplicaError,
};
use sync_engine::clock::ManualClock;
use sync_engine::dataset::MemoryDataset;
use sync_engine::engine::{Engine, Report};
use sync_engine::journal::{JournalError, JournalStore, Op};
use sync_journal_sqlite::Journal;

pub fn caps(hashes: HashKind, quota: bool, delta: Delta) -> StorageCap {
    StorageCap {
        access: Access::ReadWrite,
        delta,
        quota,
        scope: StorageScope::AppFolder,
        hashes,
        ranges: true,
        chunked_upload: false,
    }
}

/// A `MemoryReplica` that can fail once on request and counts what it was asked.
#[derive(Debug)]
pub struct Faulty {
    pub inner: MemoryReplica,
    pub changes_fault: Mutex<Option<ReplicaError>>,
    pub put_fault: Mutex<Option<PutError>>,
    /// Changes added to every page the replica answers (a whole drive's folders and online
    /// items, which the memory replica does not make).
    pub planted: Mutex<Vec<Change>>,
    pub puts: AtomicUsize,
    pub fetches: AtomicUsize,
}

fn take<T>(slot: &Mutex<Option<T>>) -> Option<T> {
    slot.lock().unwrap_or_else(PoisonError::into_inner).take()
}

impl Faulty {
    pub fn fail_next_changes(&self, error: ReplicaError) {
        *self
            .changes_fault
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(error);
    }

    pub fn fail_next_put(&self, refused: PutError) {
        *self
            .put_fault
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(refused);
    }
}

/// The replica engines share (the daemon will share one between an engine and its quota reads).
#[derive(Debug, Clone)]
pub struct Shared(pub Arc<Faulty>);

impl Replica for Shared {
    async fn changes(&self, from: Cursor) -> Result<ChangePage, ReplicaError> {
        if let Some(fault) = take(&self.0.changes_fault) {
            return Err(fault);
        }
        let mut page = self.0.inner.changes(from).await?;
        let planted = self
            .0
            .planted
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        page.changes.extend(planted.iter().cloned());
        Ok(page)
    }

    async fn fetch(&self, item: &RemoteId, range: ByteRange) -> Result<Blob, ReplicaError> {
        self.0.fetches.fetch_add(1, Ordering::Relaxed);
        self.0.inner.fetch(item, range).await
    }

    async fn put(
        &self,
        item: PutItem,
        base: BaseVersion,
    ) -> Result<(RemoteId, RemoteVersion), PutError> {
        self.0.puts.fetch_add(1, Ordering::Relaxed);
        if let Some(fault) = take(&self.0.put_fault) {
            return Err(fault);
        }
        self.0.inner.put(item, base).await
    }

    async fn remove(&self, item: &RemoteId, base: BaseVersion) -> Result<RemoteVersion, PutError> {
        self.0.inner.remove(item, base).await
    }

    async fn quota(&self) -> Result<Quota, ReplicaError> {
        self.0.inner.quota().await
    }

    fn features(&self) -> StorageCap {
        self.0.inner.features()
    }
}

pub type TestEngine = Engine<Shared, Arc<MemoryDataset>, Arc<ManualClock>>;

/// One replica, one dataset, one clock and a journal path.
pub struct World {
    pub replica: Arc<Faulty>,
    pub dataset: Arc<MemoryDataset>,
    pub clock: Arc<ManualClock>,
    pub dir: tempfile::TempDir,
    /// The journal of the engine made last, which `cut_at` cuts.
    pub cutting: Mutex<Option<Arc<Cutting>>>,
}

impl World {
    pub fn new(name: &str, features: StorageCap, page: usize) -> Self {
        Self::limited(name, features, page, None)
    }

    pub fn limited(name: &str, features: StorageCap, page: usize, limit: Option<Bytes>) -> Self {
        let mut inner = MemoryReplica::new(features, page, UnixSeconds(1_000));
        if let Some(limit) = limit {
            inner = inner.with_limit(limit);
        }
        Self {
            replica: Arc::new(Faulty {
                inner,
                changes_fault: Mutex::default(),
                put_fault: Mutex::default(),
                planted: Mutex::default(),
                puts: AtomicUsize::new(0),
                fetches: AtomicUsize::new(0),
            }),
            dataset: Arc::new(MemoryDataset::new("files", ConflictRule::KeepBoth)),
            clock: Arc::new(ManualClock::at(5_000)),
            dir: scratch(name),
            cutting: Mutex::default(),
        }
    }

    /// A fresh engine on the journal file: what a restarted process has.
    pub fn engine(&self) -> TestEngine {
        let journal = Arc::new(Cutting {
            inner: Journal::open(&self.dir.path().join("files.sqlite")).expect("journal"),
            cut: Mutex::new(None),
        });
        *self.cutting.lock().unwrap_or_else(PoisonError::into_inner) = Some(Arc::clone(&journal));
        Engine::new(
            Shared(Arc::clone(&self.replica)),
            Arc::clone(&self.dataset),
            CutHandle(journal),
            Arc::clone(&self.clock),
        )
    }

    /// Makes the `n`th journal write from now (0 is the next) of the engine made last fail
    /// before it commits, and every one after it: the process died there.
    pub fn cut_at(&self, n: usize) {
        let cutting = self
            .cutting
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
            .expect("an engine was made");
        let at = cutting.inner.writes() + n;
        *cutting.cut.lock().unwrap_or_else(PoisonError::into_inner) = Some(at);
    }

    /// Someone else writes a file to the replica.
    pub async fn remote_put(&self, path: &str, bytes: &[u8]) -> RemoteId {
        let item = PutItem {
            target: PutTarget::New(ItemPath(path.into())),
            content: Blob(bytes.to_vec()),
            hash: sync_engine::dataset::replica_hash(self.replica.inner.features().hashes, bytes),
        };
        self.replica
            .inner
            .put(item, BaseVersion::Absent)
            .await
            .expect("remote put")
            .0
    }

    /// Someone else changes a file on the replica (at its current version).
    pub async fn remote_edit(&self, id: &RemoteId, bytes: &[u8]) {
        let current = self.version_of(id).await;
        let item = PutItem {
            target: PutTarget::Existing(id.clone()),
            content: Blob(bytes.to_vec()),
            hash: sync_engine::dataset::replica_hash(self.replica.inner.features().hashes, bytes),
        };
        self.replica
            .inner
            .put(item, BaseVersion::At(current))
            .await
            .expect("remote edit");
    }

    /// Someone else deletes a file on the replica.
    pub async fn remote_remove(&self, id: &RemoteId) {
        let current = self.version_of(id).await;
        self.replica
            .inner
            .remove(id, BaseVersion::At(current))
            .await
            .expect("remote remove");
    }

    async fn version_of(&self, id: &RemoteId) -> RemoteVersion {
        let page = self
            .replica
            .inner
            .changes(Cursor::Start)
            .await
            .expect("listing");
        page.changes
            .into_iter()
            .find_map(|change| match change {
                Change::Upsert(item) if item.id == *id => Some(item.version),
                _ => None,
            })
            .expect("the item is live")
    }

    /// The replica's live files by path.
    pub async fn remote_files(&self) -> BTreeMap<String, Vec<u8>> {
        let page = self
            .replica
            .inner
            .changes(Cursor::Start)
            .await
            .expect("listing");
        let mut files = BTreeMap::new();
        for change in page.changes {
            if let Change::Upsert(item) = change {
                let blob = self
                    .replica
                    .inner
                    .fetch(&item.id, ByteRange::Whole)
                    .await
                    .expect("fetch");
                files.insert(item.path.0, blob.0);
            }
        }
        files
    }

    pub fn puts(&self) -> usize {
        self.replica.puts.load(Ordering::Relaxed)
    }

    pub fn fetches(&self) -> usize {
        self.replica.fetches.load(Ordering::Relaxed)
    }
}

pub fn sha() -> StorageCap {
    caps(HashKind::Sha256, true, Delta::Poll)
}

pub fn hashless() -> StorageCap {
    caps(HashKind::None, false, Delta::Poll)
}

/// Syncs until a cycle does nothing, at most `limit` cycles; the last report.
pub async fn settle(engine: &TestEngine, limit: usize) -> Report {
    let mut last = Report::default();
    for _ in 0..limit {
        last = engine.sync_once().await.expect("cycle");
        let did =
            last.fetched + last.uploaded + last.removed + last.discarded + last.conflicts.len();
        if did == 0 {
            break;
        }
    }
    last
}

/// The error a cut write fails with.
#[derive(Debug, thiserror::Error)]
#[error("cut before write")]
pub struct CutError;

/// A SQLite journal that can be told to die before a write.
#[derive(Debug)]
pub struct Cutting {
    inner: Journal,
    cut: Mutex<Option<usize>>,
}

/// What the engine owns: a handle on a [`Cutting`] the world keeps too.
#[derive(Debug)]
pub struct CutHandle(Arc<Cutting>);

impl JournalStore for CutHandle {
    fn writes(&self) -> usize {
        self.0.inner.writes()
    }

    fn apply(&self, ops: &[Op]) -> Result<(), JournalError> {
        let cut = *self.0.cut.lock().unwrap_or_else(PoisonError::into_inner);
        if cut.is_some_and(|at| self.0.inner.writes() >= at) {
            return Err(JournalError::Store(Box::new(CutError)));
        }
        JournalStore::apply(&self.0.inner, ops)
    }

    fn items(&self) -> Result<Vec<sync_core::JournalItem>, JournalError> {
        JournalStore::items(&self.0.inner)
    }

    fn item_by_remote(
        &self,
        id: &RemoteId,
    ) -> Result<Option<sync_core::JournalItem>, JournalError> {
        JournalStore::item_by_remote(&self.0.inner, id)
    }

    fn new_item_at(&self, path: &ItemPath) -> Result<Option<sync_core::JournalItem>, JournalError> {
        JournalStore::new_item_at(&self.0.inner, path)
    }

    fn anchor(&self) -> Result<Option<sync_core::StoredAnchor>, JournalError> {
        JournalStore::anchor(&self.0.inner)
    }

    fn tombstones(&self) -> Result<Vec<sync_core::StoredTombstone>, JournalError> {
        JournalStore::tombstones(&self.0.inner)
    }

    fn conflicts(&self) -> Result<Vec<sync_core::StoredConflict>, JournalError> {
        JournalStore::conflicts(&self.0.inner)
    }
}

/// A new empty scratch directory for one test, named after it and removed when dropped.
pub fn scratch(name: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("syncd-{name}-"))
        .tempdir()
        .expect("scratch")
}
