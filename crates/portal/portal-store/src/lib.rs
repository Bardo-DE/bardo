//! What the portal keeps between runs: the restore file and the book of what the person let apps
//! do without asking again.
//!
//! - [`RestoreStore`] is the seam over the file; [`FileStore`] is the real one (mode 0600, written
//!   atomically, a damaged file set aside) and [`MemoryStore`] is a test's.
//! - [`Keeper`] is the one book of screen-sharing and remote-control grants, behind one lock, so a
//!   change reads, edits and writes the file whole.

mod file;
mod keeper;

pub use file::{FileStore, MemoryStore, RestoreStore, StoreError};
pub use keeper::{Keeper, Kept, unix_now};
