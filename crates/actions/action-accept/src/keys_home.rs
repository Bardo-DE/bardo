//! A durable, test-only home for the cloud AI key.
//!
//! One directory outside every repository that holds what accountd needs to know the `OpenRouter`
//! account across worlds, so the owner types the key once (`dev/live/cloud-key.sh`) and every fresh
//! world reuses it.
//!
//! ```text
//! DIR/                           0700
//!   accountd.keys                0600  the key file; only accountd opens it (BARDO_ACCOUNTD_KEYS=file:...)
//!   accountd.keys.lock                 the key file's advisory lock
//!   state/bardo/accounts/registry.json         the registry (accounts and key permits; no secrets)
//!   data/bardo/accounts/providers/             the provider files `accountd add` used
//! ```
//!
//! This process never opens the key file: it only looks at its metadata. A world points accountd's
//! `BARDO_ACCOUNTD_KEYS` at it and copies the non-secret `state` and `data` trees into its scratch
//! root, so a run never changes the home.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// The key file's name inside the home.
pub const KEYS_FILE: &str = "accountd.keys";
/// The registry accountd keeps (accounts and key permits), relative to the home.
const REGISTRY: &str = "state/bardo/accounts/registry.json";

/// Why a directory cannot be the home.
#[derive(Debug, thiserror::Error)]
pub enum HomeError {
    /// The path is not absolute.
    #[error("--keys-home {0} is not an absolute path")]
    NotAbsolute(PathBuf),
    /// The directory is missing or is not one.
    #[error("--keys-home {0} is not a directory")]
    NotADirectory(PathBuf),
    /// Group or other can use the directory or the key file.
    #[error("{0} has group or other permission bits ({1:o}); chmod it to 0700 (the key file 0600)")]
    LooseMode(PathBuf, u32),
    /// The directory is inside a git work tree.
    #[error(
        "--keys-home {0} is inside a git work tree ({1}); keys must live outside every repository"
    )]
    InsideGit(PathBuf, PathBuf),
    /// The key file is not a plain file.
    #[error("{0} is not a regular file")]
    NotAFile(PathBuf),
    /// The home does not hold the account yet.
    #[error(
        "{0} does not hold the cloud key yet. Fill it once, in your own terminal (the key is typed at accountd's own prompt, echo off):\n  dev/live/cloud-key.sh {0} <accountd built with --features test-keys>\nthen run again."
    )]
    Empty(PathBuf, #[source] std::io::Error),
    /// The directory could not be read.
    #[error("could not read {0}")]
    Io(PathBuf, #[source] std::io::Error),
}

/// A checked home that holds the cloud key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeysHome(PathBuf);

fn meta_of(path: &Path) -> Result<std::fs::Metadata, HomeError> {
    std::fs::symlink_metadata(path).map_err(|e| HomeError::Io(path.to_owned(), e))
}

fn tight(path: &Path, meta: &std::fs::Metadata) -> Result<(), HomeError> {
    let mode = meta.permissions().mode() & 0o777;
    match mode & 0o077 {
        0 => Ok(()),
        _ => Err(HomeError::LooseMode(path.to_owned(), mode)),
    }
}

/// The nearest ancestor (or the directory itself) that holds a `.git`.
fn git_root(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|p| p.join(".git").exists())
        .map(Path::to_owned)
}

/// Checks a directory the way `open` does: absolute, a directory, no group or other bits, outside
/// every git work tree. Returns its real path.
///
/// # Errors
///
/// Returns [`HomeError`] when `dir` is not absolute, is not a directory, is open to group or
/// others, cannot be resolved, or lies inside a git work tree.
pub fn check_dir(dir: &Path) -> Result<PathBuf, HomeError> {
    if !dir.is_absolute() {
        return Err(HomeError::NotAbsolute(dir.to_owned()));
    }
    let meta = meta_of(dir)?;
    if !meta.is_dir() {
        return Err(HomeError::NotADirectory(dir.to_owned()));
    }
    tight(dir, &meta)?;
    let real = std::fs::canonicalize(dir).map_err(|e| HomeError::Io(dir.to_owned(), e))?;
    match git_root(&real) {
        Some(root) => Err(HomeError::InsideGit(dir.to_owned(), root)),
        None => Ok(real),
    }
}

impl KeysHome {
    /// Checks `dir` and that it holds the account.
    ///
    /// # Errors
    ///
    /// Returns [`HomeError`] when `dir` fails [`check_dir`] or does not hold a private keys file
    /// and the account registry.
    pub fn open(dir: &Path) -> Result<Self, HomeError> {
        let real = check_dir(dir)?;
        let keys = real.join(KEYS_FILE);
        let empty = |source| HomeError::Empty(dir.to_owned(), source);
        // Metadata only: the harness never opens the key file.
        let meta = std::fs::symlink_metadata(&keys).map_err(empty)?;
        if !meta.is_file() {
            return Err(HomeError::NotAFile(keys));
        }
        tight(&keys, &meta)?;
        std::fs::metadata(real.join(REGISTRY)).map_err(empty)?;
        Ok(Self(real))
    }

    /// The value for `BARDO_ACCOUNTD_KEYS`.
    #[must_use]
    pub fn keys_setting(&self) -> String {
        format!("file:{}", self.0.join(KEYS_FILE).display())
    }

    /// Copies the non-secret records into a world's scratch root: accountd's state under
    /// `.local/state` and the provider files under `data/bardo/accounts/providers`.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of creating a directory or copying a record.
    pub fn seed(&self, root: &Path) -> std::io::Result<()> {
        copy_tree(&self.0.join("state"), &root.join(".local/state"))?;
        copy_tree(&self.0.join("data"), &root.join("data"))
    }
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    if !from.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if entry.file_type()?.is_file() {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
