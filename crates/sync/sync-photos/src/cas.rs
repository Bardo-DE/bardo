//! Content addressing.
//!
//! An original is stored under the SHA-256 of its bytes, so two imports of
//! the same photo are one file, locally and on the replica, and a name can be checked against
//! the bytes it holds. Pure: the file reads and writes are in `disk`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A photo's identity: the SHA-256 of its bytes, 64 lowercase hex digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ContentId(String);

impl ContentId {
    /// The id, if `text` is one.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let ok = text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        ok.then(|| Self(text.to_owned()))
    }

    /// The id of `bytes`.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self::from_digest(Sha256::digest(bytes).as_slice())
    }

    pub(crate) fn from_digest(digest: &[u8]) -> Self {
        Self(hex::encode(digest))
    }

    /// The 64 hex digits.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Where the original is under the library's `originals` directory and under the replica's
    /// folder: `ab/abcdef...` (two digits of fan-out keep a directory small).
    #[must_use]
    pub fn rel(&self) -> String {
        format!("{}/{}", &self.0[..2], self.0)
    }

    /// The id a relative path `ab/<id>` names, if it names one (the shard must be the id's own).
    #[must_use]
    pub fn of_rel(rel: &str) -> Option<Self> {
        let (shard, name) = rel.split_once('/')?;
        let id = Self::parse(name)?;
        (id.0.starts_with(shard) && shard.len() == 2).then_some(id)
    }
}

impl TryFrom<String> for ContentId {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        Self::parse(&text).ok_or_else(|| format!("not a content id: {text:?}"))
    }
}

impl From<ContentId> for String {
    fn from(id: ContentId) -> String {
        id.0
    }
}

impl std::fmt::Display for ContentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_is_64_lowercase_hex_and_its_path_carries_its_own_shard() {
        let id = ContentId::of(b"abc");
        assert_eq!(
            id.as_str(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            id.rel(),
            "ba/ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(ContentId::of_rel(&id.rel()), Some(id.clone()));
        let upper = id.as_str().to_uppercase();
        let wrong_shard = format!("00/{id}");
        let nested = format!("ba/x/{id}");
        let bad: [&str; 7] = [
            "",
            "ba",
            &upper,
            "ba/short",
            &wrong_shard,
            &nested,
            "../etc/passwd",
        ];
        for rel in bad {
            assert_eq!(ContentId::of_rel(rel), None, "{rel:?}");
        }
        assert_eq!(ContentId::parse(&upper), None);
    }
}
