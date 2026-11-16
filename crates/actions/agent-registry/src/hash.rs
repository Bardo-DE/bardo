//! SHA-256 of an archive.

use sha2::{Digest, Sha256};

/// A SHA-256, as lower-case hex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sha256Hex(String);

/// Why a digest was not accepted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("a SHA-256 is 64 hex digits")]
pub struct DigestRefused;

impl Sha256Hex {
    /// The digest of `bytes`.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        let sum = Sha256::digest(bytes);
        Self(const_hex::encode(sum))
    }

    /// The digest of everything `reader` yields, read in chunks.
    ///
    /// # Errors
    /// The reader's error.
    pub fn of_reader(mut reader: impl std::io::Read) -> std::io::Result<Self> {
        let mut hasher = Sha256::new();
        let mut chunk = vec![0_u8; 64 * 1024];
        loop {
            let n = reader.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            hasher.update(&chunk[..n]);
        }
        Ok(Self(const_hex::encode(hasher.finalize())))
    }

    /// A digest the registry or a record wrote.
    ///
    /// # Errors
    ///
    /// Returns [`DigestRefused`] when `text` is not 64 hexadecimal digits.
    pub fn parse(text: &str) -> Result<Self, DigestRefused> {
        let text = text.trim().to_ascii_lowercase();
        if text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit()) {
            Ok(Self(text))
        } else {
            Err(DigestRefused)
        }
    }

    /// The hex.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
