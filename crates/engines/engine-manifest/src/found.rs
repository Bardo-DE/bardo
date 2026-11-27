//! What reading this computer reports: which engine is here, where it came from, and what
//! the check of a person's own install says.

use model_catalog::EngineKind;
use std::path::PathBuf;

/// Where a found program came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// A distro package, at a fixed system path.
    Package,
    /// An install of ours for everyone.
    System,
    /// An install of ours for this person.
    Person,
    /// A folder the person pointed at.
    Pointed,
}

/// An engine that is here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// Which engine.
    pub engine: EngineKind,
    /// Where it came from.
    pub origin: Origin,
    /// Its program.
    pub program: PathBuf,
    /// Its release, when the install says it.
    pub version: Option<String>,
}

/// What the check says of a program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Not a person's own install of ours: nothing to check here.
    NotChecked,
    /// It is the pinned install.
    Verified,
    /// It is not: it was changed since it was installed, or nothing pins it.
    Changed(EngineKind),
}
