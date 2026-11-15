//! The crate's integration tests: one executable, one module per former test file.
//! A separate test target needs a stated reason.
//! Separate target: `packagekit` (feature `quire-desktop`) runs a fake PackageKit on a private D-Bus.

mod desktop_entries;
mod desktop_file;
mod files;
mod mime;
mod provide;
