//! The integration tests of anyview-archive, in one binary so they link once.

#![expect(
    clippy::unwrap_used,
    reason = "tests unwrap fixtures they just built; a failure is the test failing"
)]

mod archive;
mod hostile_sevenz;
mod support;
