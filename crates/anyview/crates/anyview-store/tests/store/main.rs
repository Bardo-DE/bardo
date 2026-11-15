//! The integration tests of anyview-store, in one binary so they link once.

#![allow(clippy::unwrap_used)]

mod concurrent_read;
mod history_file;
mod place;
mod save_in_place;
mod save_safety;
mod store_writer;
mod support;
mod two_writers;
mod version_store;
