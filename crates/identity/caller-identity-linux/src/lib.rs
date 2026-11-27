//! Which program is calling: the I/O half of the caller-identity library.
//!
//! [`Pinned`] holds a process by pidfd; [`RealProc`] is the machine's `/proc` and file system
//! behind the [`caller_identity_core::ProcFs`] seam (a fixture tree under feature `fake` is the
//! other), through which [`caller_identity_core::Prober`] reads what `/proc` says of a held process
//! and [`caller_identity_core::judge`] decides. The crate is synchronous: a probe is a few small
//! reads, and an async caller wraps it in its own blocking pool (`caller-identity-dbus` offers the
//! seam). It contains no `unsafe`; the pidfd comes from rustix.

mod error;
mod owners;
mod pin;
mod procfs;
mod tablefile;
mod unit_files;
mod unix;

#[cfg(feature = "fake")]
pub mod fake;
#[cfg(feature = "fake")]
mod fixture;

pub use caller_identity_core;
pub use error::{LoadError, PinError};
#[cfg(feature = "fake")]
pub use fixture::{FixtureImage, FixtureProc, FixtureProcess};
pub use owners::OwnerView;
pub use pin::{Alive, Pinned};
pub use procfs::RealProc;
#[cfg(feature = "fake")]
pub use tablefile::DESKTOP_CALLERS_VAR;
pub use tablefile::{DROP_IN_FILES_MAX, TABLE_FILE_CAP, TableFiles};
pub use unit_files::{DROP_IN_MAX, NotUnitError, ProcessError, UNIT_FILE_CAP, unit_facts};
pub use unix::stand_stream;
