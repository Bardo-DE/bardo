//! The viewer's recently-viewed history on disk, as the launcher reads it.
//!
//! The viewer writes `history.json` whole to a temporary name, synced, and renamed over the old
//! one, so a reader sees either the old file or the new one. Readers take no lock and never write.
//!
//! Every public item is reached from this root, once.
//!
//! ```no_run
//! use anyview_store::{STORE_FOLDER, read_history};
//! use std::path::Path;
//!
//! let data = Path::new("/home/me/.local/share");
//! for viewed in read_history(&data.join(STORE_FOLDER)).entries() {
//!     println!("{}", viewed.path.as_path().display());
//! }
//! ```

#![warn(missing_docs)]

mod error;
mod history;
mod io;
mod label;
mod reader;

pub use error::{StoreError, StoreOp};
pub use history::{History, HistoryEntry};
pub use label::{ResumeLabel, resume_label};
pub use reader::{HistoryRead, read_history};

/// The folder under a person's data directory that holds the store, which the viewer writes and
/// the launcher reads (`<data>/anyview`).
pub const STORE_FOLDER: &str = "anyview";
