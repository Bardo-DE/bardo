//! The crate's integration tests: one executable, one module per topic.
//! `fake_issuer` needs the `io` feature.

#[cfg(feature = "io")]
mod fake_issuer;
