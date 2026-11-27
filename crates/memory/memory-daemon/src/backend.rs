//! `SystemBackend`: the real seams. The key store is the Secret Service, the log is `SQLCipher`,
//! the vault is sealed or plain per Space, the models are inferd's.

use crate::registry::SpaceRegistry;
use base_clock::SystemClock;
use inference_client::DbusTransport;
use memory_core::{Dirs, ReplicaId, SpaceId, SpaceMeta, VaultKind};
use memory_eventlog::SqliteLog;
use memory_files::{PlainDir, SealedDir, SpaceVault};
use memory_infer::{InferdConsolidator, InferdEmbedder};
use memory_recall::{Embedder, ExactScan, Fts5, Index, IndexError};
use memory_seal::{DbKey, KeyStore, Oo7Keys, Purpose, SpaceKey, derive};
use memory_service::{Backend, BackendError, Listing, RegistryError};
use memory_tidy::Consolidator;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;

/// memoryd's seams over the real system: `SQLCipher` files under `dirs`, sealed or plain vaults,
/// the OS's randomness and clock.
///
/// The key store, the embedder and the consolidator are parameters whose defaults are the real ones
/// (the Secret Service, inferd); a test names its own and keeps everything else real.
#[derive(Debug)]
pub struct SystemBackend<
    K = Oo7Keys,
    E = InferdEmbedder<DbusTransport>,
    C = InferdConsolidator<DbusTransport>,
> {
    dirs: Dirs,
    keys: K,
    embedder: E,
    consolidator: C,
    clock: SystemClock,
    registry: SpaceRegistry,
}

impl SystemBackend {
    /// Over `dirs` and one inferd connection shared by the embedder and the consolidator.
    #[must_use]
    pub fn new(dirs: Dirs, inferd: Arc<DbusTransport>, card: memory_store::EmbedderCard) -> Self {
        Self::with(
            dirs,
            Oo7Keys,
            InferdEmbedder::new(Arc::clone(&inferd), card),
            InferdConsolidator::new(inferd),
        )
    }
}

impl<K, E, C> SystemBackend<K, E, C> {
    /// Over `dirs` with the given seams.
    pub fn with(dirs: Dirs, keys: K, embedder: E, consolidator: C) -> Self {
        Self {
            dirs,
            keys,
            embedder,
            consolidator,
            clock: SystemClock,
            registry: SpaceRegistry::Closed,
        }
    }

    /// Where the desktop-wide Spaces are listed (closed until this is called: an app opens no
    /// desktop-wide Space).
    #[must_use]
    pub fn with_registry(mut self, registry: SpaceRegistry) -> Self {
        self.registry = registry;
        self
    }

    /// The directories.
    pub fn dirs(&self) -> &Dirs {
        &self.dirs
    }
}

/// A `SQLCipher` file opened with `key` (`PRAGMA key`, then `secure_delete` so removed rows do not
/// linger in free pages). A wrong key shows at the first query, which the caller makes.
fn open_keyed(path: &Path, key: &DbKey) -> Result<Connection, IndexError> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "key", key.pragma())?;
    conn.pragma_update(None, "secure_delete", "ON")?;
    // No temporary files: the sandbox has no directory for them.
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    Ok(conn)
}

/// `index.db`: FTS5 and the vectors in one file, created with `memory_recall::SCHEMA_V1` when it is
/// new. The two halves hold a connection each (the file is theirs alone and they never run at the
/// same time).
fn open_index_file<E: Embedder>(
    path: &Path,
    key: &DbKey,
    embedder: &E,
) -> Result<Index<ExactScan>, IndexError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| IndexError::Sqlite(e.to_string()))?;
    }
    let lexical = open_keyed(path, key)?;
    // SQLCipher reads nothing until the first query: a wrong key shows here.
    let version: u32 = lexical.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let fts = Fts5::new(lexical);
    if version == 0 {
        fts.create()?;
        fts.connection().pragma_update(None, "user_version", 1)?;
    }
    let vectors = ExactScan::new(open_keyed(path, key)?, embedder.card().clone());
    Ok(Index::new(fts, vectors))
}

fn gone_is_fine(result: std::io::Result<()>) -> Result<(), BackendError> {
    match result {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(BackendError::Remove(e.to_string()))
        }
        _ => Ok(()),
    }
}

impl<K: KeyStore, E: Embedder, C: Consolidator> Backend for SystemBackend<K, E, C> {
    type Keys = K;
    type Log = SqliteLog;
    type Files = SpaceVault;
    type Index = Index<ExactScan>;
    type Embedder = E;
    type Consolidator = C;
    type Clock = SystemClock;

    fn keys(&self) -> &K {
        &self.keys
    }

    fn embedder(&self) -> &E {
        &self.embedder
    }

    fn consolidator(&self) -> &C {
        &self.consolidator
    }

    fn clock(&self) -> &SystemClock {
        &self.clock
    }

    fn random(&self) -> Result<[u8; 16], BackendError> {
        Ok(base_random::bytes()?)
    }

    fn remove_space(&self, space: &SpaceId) -> Result<(), BackendError> {
        for dir in [
            self.dirs.space(space),
            self.dirs.index_dir(space),
            self.dirs.edit(space),
        ] {
            gone_is_fine(std::fs::remove_dir_all(dir))?;
        }
        Ok(())
    }

    fn is_listed(
        &self,
        space: &SpaceId,
    ) -> impl Future<Output = Result<Listing, RegistryError>> + Send {
        self.registry.is_listed(space)
    }

    fn open_log(
        &self,
        space: &SpaceId,
        replica: ReplicaId,
        key: &SpaceKey,
    ) -> Result<SqliteLog, BackendError> {
        let db_key = DbKey::of(&derive(key, space, Purpose::Eventlog));
        let digest = derive(key, space, Purpose::Digest);
        Ok(SqliteLog::open_for(
            &self.dirs.events_db(space),
            &db_key,
            space,
            replica,
            &digest,
        )?)
    }

    fn open_files(&self, meta: &SpaceMeta, key: &SpaceKey) -> Result<SpaceVault, BackendError> {
        let plain = PlainDir::new(self.dirs.space(&meta.id));
        Ok(match meta.vault {
            VaultKind::Plain => SpaceVault::Plain(plain),
            VaultKind::Sealed => SpaceVault::Sealed(SealedDir::new(
                plain,
                meta.id.clone(),
                derive(key, &meta.id, Purpose::Files),
            )),
        })
    }

    fn open_index(
        &self,
        space: &SpaceId,
        key: &SpaceKey,
    ) -> Result<Index<ExactScan>, BackendError> {
        let db_key = DbKey::of(&derive(key, space, Purpose::Index));
        open_index_file(&self.dirs.index_db(space), &db_key, &self.embedder)
            .map_err(|e| BackendError::Index(e.into()))
    }
}
