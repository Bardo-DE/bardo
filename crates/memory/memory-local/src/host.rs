//! Opening the service over a root, and keeping `spaces.toml` (what memoryd's daemon does at
//! start and after each request).

use crate::backend::LocalBackend;
use base_fs::atomic::AtomicWrite;
use base_time::Clock;
use memory_core::{ReplicaId, RuleSet, SpaceId, SpaceMeta, VaultKind};
use memory_host::read_config;
use memory_recall::Embedder;
use memory_service::{
    Backend, ConfigError, MemoryService, SpacesFile, spaces_from_toml, spaces_to_toml,
};
use memory_tidy::Consolidator;

/// Why the root could not be opened or written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LocalError {
    /// `spaces.toml` could not be read or written as a file.
    #[error("spaces.toml: {0}")]
    File(String),
    /// `spaces.toml` is not valid.
    #[error("spaces.toml: {0}")]
    Config(#[from] ConfigError),
    /// A Space with this id is already there.
    #[error("the Space already exists")]
    Exists,
}

/// The service over `backend`, with every Space in the root's `spaces.toml` registered (none
/// when the file is new). No Space opens, and no key is asked for, until the first request.
///
/// # Errors
///
/// [`LocalError`] when the spaces file cannot be read or does not parse.
pub fn open<Clk: Clock, E: Embedder, C: Consolidator>(
    backend: LocalBackend<Clk, E, C>,
    rules: RuleSet,
) -> Result<MemoryService<LocalBackend<Clk, E, C>>, LocalError> {
    let path = backend.dirs().spaces_toml();
    let found = match read_config(&path).map_err(|e| LocalError::File(e.to_string()))? {
        Some(text) => spaces_from_toml(&text)?,
        None => SpacesFile::default(),
    };
    let service = MemoryService::new(backend, rules);
    found
        .spaces
        .into_iter()
        .for_each(|meta| service.register(meta));
    Ok(service)
}

/// Writes `spaces.toml` from what the service knows (call after creating a Space).
///
/// # Errors
///
/// [`LocalError`] when the spaces cannot be written as text or the file cannot be written.
pub fn save_spaces<B: Backend>(
    dirs: &memory_core::Dirs,
    service: &MemoryService<B>,
) -> Result<(), LocalError> {
    let text = spaces_to_toml(&SpacesFile {
        spaces: service.metas(),
    })?;
    let path = dirs.spaces_toml();
    AtomicWrite::PRIVATE
        .write(&path, text.as_bytes())
        .map_err(|e| LocalError::File(e.to_string()))
}

/// Registers a new Space (created now, with a fresh replica id) and saves `spaces.toml`.
///
/// # Errors
///
/// [`LocalError::Exists`] when the space is already there; another [`LocalError`] when the space
/// cannot be made.
pub fn create_space<Clk: Clock, E: Embedder, C: Consolidator>(
    service: &MemoryService<LocalBackend<Clk, E, C>>,
    id: SpaceId,
    vault: VaultKind,
) -> Result<SpaceMeta, LocalError> {
    if service.metas().iter().any(|m| m.id == id) {
        return Err(LocalError::Exists);
    }
    let backend = service.backend();
    let mut replica = [0u8; 16];
    replica.copy_from_slice(&backend.random());
    let meta = SpaceMeta {
        id,
        created: backend.clock().now(),
        replica: ReplicaId(replica),
        vault,
        format: 1,
    };
    service.register(meta.clone());
    save_spaces(backend.dirs(), service)?;
    Ok(meta)
}
