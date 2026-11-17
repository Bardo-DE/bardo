//! A scratch directory every path the daemon touches lives under.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT: AtomicU32 = AtomicU32::new(0);

/// A directory under the system temp directory, readable and searchable by everyone (the
/// observer test runs a client as another user), removed on drop.
#[derive(Debug)]
pub struct Scratch(PathBuf);

impl Scratch {
    /// A fresh directory named for `label` and this process.
    ///
    /// # Panics
    ///
    /// Panics when the directory cannot be created or opened up: a harness that cannot make its
    /// scratch space cannot run.
    pub fn new(label: &str) -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "key-remap-devtest-{}-{n}-{}",
            std::process::id(),
            label.replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "_")
        ));
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("creating {}: {e}", dir.display()));
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755))
            .unwrap_or_else(|e| panic!("opening up {}: {e}", dir.display()));
        Self(dir)
    }

    /// The directory.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }

    /// A subdirectory, created readable and searchable by everyone.
    ///
    /// # Panics
    ///
    /// Panics when the directory cannot be created or opened up.
    #[must_use]
    pub fn dir(&self, name: &str) -> PathBuf {
        let dir = self.0.join(name);
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("creating {}: {e}", dir.display()));
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755))
            .unwrap_or_else(|e| panic!("opening up {}: {e}", dir.display()));
        dir
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scratch_is_open_to_everyone_and_removed_on_drop() {
        let path = {
            let s = Scratch::new("own test/1");
            let sub = s.dir("run");
            let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
            assert_eq!((mode(s.path()), mode(&sub)), (0o755, 0o755));
            s.path().to_path_buf()
        };
        assert!(!path.exists());
        assert!(path.to_string_lossy().ends_with("own_test_1"));
    }
}
