//! A scratch directory for a test: unique to the process and the call, removed on drop.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// A fresh, empty directory under the system temp dir.
pub(crate) struct Dir(PathBuf);

impl Dir {
    pub(crate) fn new() -> Dir {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("shell-host-test-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).expect("temp dir");
        Dir(path)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
