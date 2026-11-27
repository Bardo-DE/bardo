//! The real file system under a [`ProcFs`]: the machine's `/proc` and the files its answers point
//! at.

use crate::owners::OwnerView;
use base_fs::{ReadCap, ReadError};
use caller_identity_core::{FileTrust, Follow, ProcFs, trust_of_mode};
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

fn trust_of_meta(meta: &std::fs::Metadata, owners: OwnerView) -> FileTrust {
    trust_of_mode(owners.owner(meta.uid()), meta.mode())
}

fn capped(path: &Path, cap: u64) -> io::Result<Vec<u8>> {
    base_fs::read_capped(path, ReadCap::bytes(cap)).map_err(|error| match error {
        ReadError::NotFound { .. } => io::Error::from(io::ErrorKind::NotFound),
        ReadError::Io { source, .. } => source,
        ReadError::TooBig { .. } => {
            io::Error::new(io::ErrorKind::InvalidData, "larger than the cap")
        }
        other => io::Error::new(io::ErrorKind::InvalidData, other),
    })
}

/// The real thing: a `/proc` mount and the machine's file system, with owners read as this
/// process's user namespace shows them ([`OwnerView`]).
#[derive(Debug, Clone)]
pub struct RealProc {
    root: PathBuf,
    owners: OwnerView,
}

impl RealProc {
    /// The system's `/proc`: the one fixed path of the crate, named here and nowhere else.
    #[must_use]
    pub fn system() -> RealProc {
        RealProc::at(PathBuf::from("/proc"))
    }

    /// A `/proc` mounted at `root`; the namespace is read from its `self/uid_map` and
    /// `sys/kernel/overflowuid`.
    #[must_use]
    pub fn at(root: PathBuf) -> RealProc {
        let read = |path: PathBuf| {
            capped(&path, 4096)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .unwrap_or_default()
        };
        let owners = OwnerView::of(
            &read(root.join("self/uid_map")),
            &read(root.join("sys/kernel/overflowuid")),
        );
        RealProc { root, owners }
    }

    /// How this process's namespace shows file owners.
    #[must_use]
    pub fn owners(&self) -> OwnerView {
        self.owners
    }
}

impl ProcFs for RealProc {
    fn proc_root(&self) -> &Path {
        &self.root
    }

    fn read_capped(&self, path: &Path, cap: u64) -> io::Result<Vec<u8>> {
        capped(path, cap)
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        std::fs::read_link(path)
    }

    fn trust_of(&self, path: &Path, follow: Follow) -> io::Result<FileTrust> {
        match follow {
            Follow::No => std::fs::symlink_metadata(path),
            Follow::Yes => std::fs::metadata(path),
        }
        .map(|m| trust_of_meta(&m, self.owners))
    }

    fn list_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        std::fs::read_dir(path)?
            .map(|entry| entry.map(|e| e.path()))
            .collect()
    }
}
