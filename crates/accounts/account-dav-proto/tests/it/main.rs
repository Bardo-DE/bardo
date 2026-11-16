//! The crate's integration tests: one executable, one module per topic. The crate depends on
//! no porter crate, so account-fake's module guard is not run here.

#[cfg(feature = "discover")]
mod discover;
mod fixtures;
mod mailo;
