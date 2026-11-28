//! Reads what the `xdg-core` parsers take: the desktop entries under the `applications` folders,
//! the shared MIME database and the app associations under the data and config folders, and this
//! process's XDG environment.
//!
//! Every function takes the folders as an argument; only [`process_xdg_env`] reads the
//! environment. A file that cannot be read is skipped (and counted where a caller can use the
//! count), never an error for the rest.

mod entries;
mod env;
mod loader;
mod walk;

pub use entries::load_entries;
pub use env::process_xdg_env;
pub use loader::{LoadError, Loaded, XdgDirs, load_associations, load_mime_db, mimeapps_paths};
pub use walk::{EntryFile, first_of_each_id, walk_applications};
