//! The shared test rig: a world of one replica, one dataset, one clock and a journal file.

use account_capability::{Access, Delta, HashKind, StorageCap, StorageScope};
use base_time::UnixSeconds;
use std::sync::Arc;
use sync_core::{
    BaseVersion, Blob, ByteRange, ChangePage, ConflictRule, Cursor, ItemPath, MemoryReplica,
    PutError, PutItem, PutTarget, Quota, RemoteId, RemoteVersion, Replica, ReplicaError,
};
use sync_engine::clock::ManualClock;
use sync_engine::dataset::{MemoryDataset, replica_hash};
use sync_engine::engine::Engine;
use sync_journal_sqlite::Journal;

pub fn sha() -> StorageCap {
    StorageCap {
        access: Access::ReadWrite,
        delta: Delta::Poll,
        quota: true,
        scope: StorageScope::AppFolder,
        hashes: HashKind::Sha256,
        ranges: true,
        chunked_upload: false,
    }
}

/// The replica the engine and the test share.
#[derive(Debug, Clone)]
pub struct Shared(pub Arc<MemoryReplica>);

impl Replica for Shared {
    async fn changes(&self, from: Cursor) -> Result<ChangePage, ReplicaError> {
        self.0.changes(from).await
    }

    async fn fetch(&self, item: &RemoteId, range: ByteRange) -> Result<Blob, ReplicaError> {
        self.0.fetch(item, range).await
    }

    async fn put(
        &self,
        item: PutItem,
        base: BaseVersion,
    ) -> Result<(RemoteId, RemoteVersion), PutError> {
        self.0.put(item, base).await
    }

    async fn remove(&self, item: &RemoteId, base: BaseVersion) -> Result<RemoteVersion, PutError> {
        self.0.remove(item, base).await
    }

    async fn quota(&self) -> Result<Quota, ReplicaError> {
        self.0.quota().await
    }

    fn features(&self) -> StorageCap {
        self.0.features()
    }
}

pub type TestEngine = Engine<Shared, Arc<MemoryDataset>, Arc<ManualClock>>;

/// One replica, one dataset, one clock and a journal path.
pub struct World {
    pub replica: Arc<MemoryReplica>,
    pub dataset: Arc<MemoryDataset>,
    pub clock: Arc<ManualClock>,
    pub dir: tempfile::TempDir,
}

impl World {
    pub fn new(name: &str, features: StorageCap, page: usize) -> Self {
        Self {
            replica: Arc::new(MemoryReplica::new(features, page, UnixSeconds(1_000))),
            dataset: Arc::new(MemoryDataset::new("files", ConflictRule::KeepBoth)),
            clock: Arc::new(ManualClock::at(5_000)),
            dir: tempfile::Builder::new()
                .prefix(&format!("syncd-{name}-"))
                .tempdir()
                .expect("scratch"),
        }
    }

    /// A fresh engine on the journal file.
    pub fn engine(&self) -> TestEngine {
        let journal = Journal::open(&self.dir.path().join("files.sqlite")).expect("journal");
        Engine::new(
            Shared(Arc::clone(&self.replica)),
            Arc::clone(&self.dataset),
            journal,
            Arc::clone(&self.clock),
        )
    }

    /// Someone else writes a file to the replica.
    pub async fn remote_put(&self, path: &str, bytes: &[u8]) -> RemoteId {
        let item = PutItem {
            target: PutTarget::New(ItemPath(path.into())),
            content: Blob(bytes.to_vec()),
            hash: replica_hash(self.replica.features().hashes, bytes),
        };
        self.replica
            .put(item, BaseVersion::Absent)
            .await
            .expect("remote put")
            .0
    }
}
