//! Keys: the per-Space key, the purpose subkeys derived from it, and the database key.

use memory_core::SpaceId;
use std::fmt;
use zeroize::Zeroize;

/// One Space's root key. Zeroised on drop, never serialised, never printed.
#[derive(Clone, PartialEq, Eq)]
pub struct SpaceKey([u8; 32]);

impl SpaceKey {
    /// A key from 32 bytes (the key store's, or a test's).
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// A fresh random key.
    ///
    /// # Errors
    ///
    /// The [`base_random::RandomError`] when the system gives no random bytes.
    pub fn generate() -> Result<Self, base_random::RandomError> {
        base_random::bytes().map(Self)
    }

    /// The raw bytes, for the key store only.
    #[must_use]
    pub fn expose(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Drop for SpaceKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for SpaceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SpaceKey(<redacted>)")
    }
}

/// What a subkey is for. Each purpose has its own derivation context, so a key for one use is
/// never valid for another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Purpose {
    /// The event log's `SQLCipher` key.
    Eventlog,
    /// The recall index's `SQLCipher` key.
    Index,
    /// Sealing memory files.
    Files,
    /// The keyed digest of event bodies.
    Digest,
}

impl Purpose {
    /// The `blake3::derive_key` context string.
    #[must_use]
    pub const fn context(self) -> &'static str {
        match self {
            Purpose::Eventlog => "bardo-memory 1 eventlog",
            Purpose::Index => "bardo-memory 1 index",
            Purpose::Files => "bardo-memory 1 files",
            Purpose::Digest => "bardo-memory 1 digest",
        }
    }
}

/// A key derived for one purpose in one Space.
#[derive(Clone, PartialEq, Eq)]
pub struct SubKey([u8; 32]);

impl SubKey {
    /// The raw bytes, for the primitives that take a key.
    #[must_use]
    pub fn expose(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Drop for SubKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for SubKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SubKey(<redacted>)")
    }
}

/// The subkey of `key` for `purpose` in `space`: `blake3::derive_key` over the purpose's
/// context, with the key and the length-prefixed Space id as the key material.
#[must_use]
pub fn derive(key: &SpaceKey, space: &SpaceId, purpose: Purpose) -> SubKey {
    let mut hasher = blake3::Hasher::new_derive_key(purpose.context());
    hasher.update(key.expose());
    hasher.update(
        &u32::try_from(space.as_str().len())
            .unwrap_or(u32::MAX)
            .to_be_bytes(),
    );
    hasher.update(space.as_str().as_bytes());
    SubKey(*hasher.finalize().as_bytes())
}

/// A `SQLCipher` raw key, as the pragma wants it.
#[derive(Clone, PartialEq, Eq)]
pub struct DbKey(String);

impl DbKey {
    /// The database key for a subkey (derive it with [`Purpose::Eventlog`] or
    /// [`Purpose::Index`]).
    #[must_use]
    pub fn of(sub: &SubKey) -> Self {
        Self(const_hex::encode(sub.expose()))
    }

    /// The value of `PRAGMA key`: `x'<64 hex digits>'`, `SQLCipher`'s raw-key form.
    #[must_use]
    pub fn pragma(&self) -> String {
        format!("x'{}'", self.0)
    }
}

impl Drop for DbKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for DbKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DbKey(<redacted>)")
    }
}
