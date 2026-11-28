//! The crate's integration tests: one executable, one module per subject.
//! A separate test target needs a stated reason.
//! `packagekit` (feature `desktop`) runs a fake `PackageKit` on a private D-Bus.

#[cfg(feature = "desktop")]
mod packagekit;
mod provide;
