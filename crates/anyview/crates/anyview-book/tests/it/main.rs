//! The integration tests of anyview-book, in one binary so they link once.

#![allow(
    clippy::unwrap_used,
    reason = "a test that cannot unwrap its fixture has already failed"
)]

mod books;
mod hostile;
mod support;
