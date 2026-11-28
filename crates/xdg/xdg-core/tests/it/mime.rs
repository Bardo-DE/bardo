//! The file-type layer (`xdg_core::mime`): one module per former test file. All text is inline
//! fixtures; nothing reads the real system.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "tests may unwrap")]

use file_types::{FileName, MimeType};

mod apps;
mod db;
mod globs;

/// A type from text.
fn mt(text: &str) -> MimeType {
    MimeType::new(text).unwrap()
}

/// A file name from text.
fn name(text: &str) -> FileName {
    FileName::new(text).unwrap()
}
