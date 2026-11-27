//! The file system as the prober sees it: `/proc` and the files its answers point at. A trait,
//! so the tests read a fixture tree and the I/O crate reads the kernel.

use crate::facts::{FileTrust, Writer};
use crate::ids::Uid;
use std::io;
use std::path::{Path, PathBuf};

/// Whether [`ProcFs::trust_of`] looks at a final link itself or at what it points to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Follow {
    /// The path itself: a final link is not followed.
    No,
    /// The file behind the path, taken from the open inode (so a deleted image is judged by the
    /// file that is running, not by whatever is at its path).
    Yes,
}

/// The file-system view the prober and the table loader read through.
///
/// Every path is a path of the file system. The process files live under [`ProcFs::proc_root`]
/// (`<root>/1234/cgroup`). Callers build every path themselves from numbers and fixed names;
/// nothing here is handed a path by a peer.
pub trait ProcFs: Send + Sync {
    /// Where `/proc` is mounted: the system's, or a fixture's.
    fn proc_root(&self) -> &Path;
    /// The bytes of the file `path`, failing with `InvalidData` over `cap` bytes.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of the read, or `InvalidData` when the file is over `cap` bytes.
    fn read_capped(&self, path: &Path, cap: u64) -> io::Result<Vec<u8>>;
    /// Where the link `path` points.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of the read.
    fn read_link(&self, path: &Path) -> io::Result<PathBuf>;
    /// The trust of `path`, looking at a final link or through it as `follow` says.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of the lookup.
    fn trust_of(&self, path: &Path, follow: Follow) -> io::Result<FileTrust>;
    /// The entries of the directory `path`, as paths.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of the read.
    fn list_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>>;
}

/// The trust of a file owned by `uid` with permission bits `mode`: root-only when root owns it
/// and neither group nor others can write.
#[must_use]
pub fn trust_of_mode(uid: u32, mode: u32) -> FileTrust {
    const GROUP_WRITE: u32 = 0o020;
    const OTHER_WRITE: u32 = 0o002;
    if uid != 0 {
        FileTrust::Writable(Writer::Owner(Uid::new(uid)))
    } else if mode & OTHER_WRITE != 0 {
        FileTrust::Writable(Writer::Other)
    } else if mode & GROUP_WRITE != 0 {
        FileTrust::Writable(Writer::Group)
    } else {
        FileTrust::RootOnly
    }
}
