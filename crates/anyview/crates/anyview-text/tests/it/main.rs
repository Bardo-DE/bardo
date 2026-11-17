//! The integration tests of anyview-text, in one binary so they link once.

#![allow(clippy::unwrap_used, reason = "tests may unwrap")]

mod hostile_sheets;
mod peek;
mod sheets;
mod support;
mod windowing;
