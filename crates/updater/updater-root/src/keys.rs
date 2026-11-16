//! The trust anchor: the minisign public key releases are signed with.

use std::path::Path;
use updater_core::PublicKey;

/// A key compiled in at build time (`BARDO_UPDATER_RELEASE_PUBKEY`), used when no key file exists.
pub const COMPILED_KEY: Option<&str> = option_env!("BARDO_UPDATER_RELEASE_PUBKEY");

/// Why there is no usable key.
#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    /// No key file and none compiled in.
    #[error("this computer has no key to check updates with")]
    Missing,
    /// A key is there but cannot be read.
    #[error("the key to check updates with cannot be read")]
    Unreadable,
}

/// Loads the key from `path`, or the compiled-in one when `path` does not exist.
///
/// # Errors
///
/// Returns [`KeyError::Unreadable`] when the file exists and is not a readable key, and
/// [`KeyError::Missing`] when there is no file and no compiled-in key.
pub fn load_key(path: &Path) -> Result<PublicKey, KeyError> {
    match std::fs::read_to_string(path) {
        Ok(text) => PublicKey::parse(&text).ok().ok_or(KeyError::Unreadable),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => from_compiled(COMPILED_KEY),
        Err(_) => Err(KeyError::Unreadable),
    }
}

/// The compiled-in key text, if any, parsed.
///
/// # Errors
///
/// Returns [`KeyError::Missing`] when there is no compiled-in text, and [`KeyError::Unreadable`]
/// when it is not a key.
pub fn from_compiled(compiled: Option<&str>) -> Result<PublicKey, KeyError> {
    let text = compiled
        .filter(|t| !t.trim().is_empty())
        .ok_or(KeyError::Missing)?;
    PublicKey::parse(text).ok().ok_or(KeyError::Unreadable)
}
