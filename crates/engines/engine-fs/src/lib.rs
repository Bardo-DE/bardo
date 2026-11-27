//! Reading this computer for the engine manifest. Files only: nothing here runs a program it
//! finds, and every path and root is an argument (no environment, no guessing a home directory).
//! What the readings are made into (`Host`, `Roots`, `Record`, `Found`, `Verdict`) is
//! `engine-manifest`'s.
//!
//! - [`read_host`] and [`read_gpus`] make `engine-manifest`'s `Host` from the kernel's files and
//!   `os-release`.
//! - [`read_record`] and [`write_record`] read and write the record an install leaves.
//! - [`Finder`] says which engine is already here and where it came from.
//! - [`Verifier`] is the check before a person's own install starts: it must still be the pinned
//!   tree. [`seal_of`] and [`entry_digests`] are the walks it makes.

mod found;
mod host;
mod record;
mod seal;
mod verify;

pub use found::{Finder, pointed};
pub use host::{read_gpus, read_host};
pub use record::{read_record, write_record};
pub use seal::{SealError, file_digest, seal_of};
pub use verify::{Verifier, entry_digests};
