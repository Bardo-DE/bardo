//! The crate's integration tests: one executable, one module per former test file.
//! A separate test target needs a stated reason.
//! Separate target: `live` (feature `live`) runs the settings service on a private D-Bus.

mod appearance_file;
mod damaged_files;
mod derive;
mod doc_tables;
mod hot_corners;
mod published_parts;
mod rows;
mod schema;
mod spaces_file;
mod spaces_storage;
mod store;
mod support;
mod user_style;
mod watch;
mod when;
mod writer;
