//! The library's file I/O: hashing a file in chunks and the atomic writes every part of the
//! library uses. The values they work on are in `cas`.

use crate::cas::ContentId;
use base_fs::atomic::AtomicWrite;
use sha2::{Digest, Sha256};
use std::io::{self, Read};
use std::path::Path;

/// The id and size of the file at `path`, read in chunks (a photo is never held whole).
///
/// # Errors
///
/// Returns the I/O error when the file cannot be opened or read.
pub fn hash_file(path: &Path) -> io::Result<(ContentId, u64)> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut size = 0u64;
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        size += n as u64;
    }
    Ok((ContentId::from_digest(hasher.finalize().as_slice()), size))
}

/// Writes `bytes` to `target` so that a reader sees the old file or the whole new one.
///
/// It goes through the accounts service's one atomic writer: a staging file (a dot file named `.tmp-*`, which no scan lists)
/// beside it, synced, renamed over it. Missing directories are made.
///
/// # Errors
///
/// Returns the I/O error when the file cannot be written.
pub fn write_atomic(target: &Path, bytes: &[u8]) -> io::Result<()> {
    AtomicWrite::SHARED.write(target, bytes)
}

/// Copies `source` to `target` the same way.
///
/// # Errors
///
/// Returns the I/O error when the file cannot be copied.
pub fn copy_atomic(source: &Path, target: &Path) -> io::Result<()> {
    AtomicWrite::SHARED.copy(source, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::scratch;

    #[test]
    fn hashing_a_file_matches_hashing_its_bytes_and_counts_them() {
        let dir_guard = scratch("cas-hash");
        let dir = dir_guard.path().to_owned();
        let bytes: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(dir.join("big"), &bytes).expect("file");
        assert_eq!(
            hash_file(&dir.join("big")).expect("hash"),
            (ContentId::of(&bytes), 200_000)
        );
        hash_file(&dir.join("missing")).unwrap_err();
    }

    #[test]
    fn an_atomic_write_replaces_whole_and_leaves_no_temp_file() {
        let dir_guard = scratch("cas-atomic");
        let dir = dir_guard.path().to_owned();
        let target = dir.join("a/b/file");
        write_atomic(&target, b"one").expect("first");
        write_atomic(&target, b"two").expect("second");
        assert_eq!(std::fs::read(&target).expect("read"), b"two");
        std::fs::write(dir.join("src"), b"three").expect("src");
        copy_atomic(&dir.join("src"), &target).expect("copy");
        assert_eq!(std::fs::read(&target).expect("read"), b"three");
        assert!(copy_atomic(&dir.join("missing"), &target).is_err());
        assert_eq!(
            std::fs::read(&target).expect("read"),
            b"three",
            "a failed copy changes nothing"
        );
        let names: Vec<_> = std::fs::read_dir(target.parent().expect("dir"))
            .expect("dir")
            .map(|e| e.expect("entry").file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("file")]);
    }
}
