//! Where the actions service keeps agents: one directory it owns, apart from the person's own files.
//!
//! ```text
//! <root>/installed/<id>/<version>/files/        what the archive unpacked to
//! <root>/installed/<id>/<version>/installed.toml  what was installed and how it was checked
//! <root>/pins/<id>/<version>.sha256               the digest first seen for a version with none declared
//! <root>/state/<id>/home/                         the agent's own home inside its sandbox
//! <root>/state/<id>/offered.toml                  what it last offered (see `offered`)
//! ```
//!
//! The agent signs in inside `home` and keeps its login there. Nothing is copied into it from the
//! person's real home. This is the layout as paths and touches no disk; listing and reading what is
//! there is `bardo-agents`.

use crate::slug::Slug;
use std::path::{Path, PathBuf};

/// The agents directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentsDir(PathBuf);

impl AgentsDir {
    /// The directory at `root` (made when something is installed).
    #[must_use]
    pub fn at(root: &Path) -> Self {
        Self(root.to_owned())
    }

    /// The directory with one directory per installed agent.
    #[must_use]
    pub fn all_installed(&self) -> PathBuf {
        self.0.join("installed")
    }

    /// The directory with one directory per installed version of `id`.
    #[must_use]
    pub fn installed_of(&self, id: &Slug) -> PathBuf {
        self.all_installed().join(id.as_str())
    }

    /// Where version `version` of `id` is installed.
    #[must_use]
    pub fn installed(&self, id: &Slug, version: &Slug) -> PathBuf {
        self.installed_of(id).join(version.as_str())
    }

    /// The file that keeps the digest first seen for version `version` of `id`, when the registry
    /// declared none. It outlives the install, so a later install is compared with it.
    #[must_use]
    pub fn pin(&self, id: &Slug, version: &Slug) -> PathBuf {
        self.0
            .join("pins")
            .join(id.as_str())
            .join(format!("{}.sha256", version.as_str()))
    }

    /// The agent's home inside its sandbox.
    #[must_use]
    pub fn home(&self, id: &Slug) -> PathBuf {
        self.0.join("state").join(id.as_str()).join("home")
    }

    /// The file with the record of what `id` last offered.
    #[must_use]
    pub fn offered(&self, id: &Slug) -> PathBuf {
        self.0
            .join("state")
            .join(id.as_str())
            .join(crate::offered::FILE)
    }
}
