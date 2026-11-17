//! The integration tests of anyview-store, in one binary so they link once.

#![allow(
    clippy::unwrap_used,
    reason = "a failed unwrap in a test is the failure report"
)]

mod history_file;
