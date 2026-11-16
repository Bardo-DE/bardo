//! The one place the desktop reads and writes files that must never be seen half written or read
//! without a limit, usable without any UI, bus or runtime.
//!
//! - [`read_capped`] / [`read_text_capped`]: open a regular file without blocking on a FIFO, take
//!   at most the caller's [`ReadCap`], and say apart "there is no such file" ([`ReadError::NotFound`])
//!   from every other failure.
//! - [`write_atomic`]: the one writer. A uniquely named staging file (`.<name>.<pid>.<n>.tmp`)
//!   beside the target, `sync_all`, a rename, and an fsync of the directory; the target's mode is
//!   kept, a symlink is written through to its target, and two writers of one path in a process
//!   take turns. Everything below writes through the same core.
//! - [`update_atomic`]: read, edit and write back while holding the file's lock (in this process
//!   and, through a sidecar `flock`, between programs).
//! - [`move_aside`]: a damaged file kept under a timestamped `.bad` name before anything is
//!   written in its place.
//! - [`atomic::AtomicWrite`]: the same write as a value of options (owner-only or umask for a new
//!   file, parent folders made or not; the symlink, mode and lock rules are `write_atomic`'s),
//!   which can also stage and commit in two steps; [`atomic::AtomicFile`]: one small file kept with
//!   its backup. With the `book` feature, [`book::commit`] saves a kept document on the blocking pool.

mod aside;
pub mod atomic;
#[cfg(feature = "book")]
pub mod book;
mod cap;
mod flock;
mod link;
mod lock;
mod read;
mod update;
mod write;

pub use aside::move_aside;
pub use cap::ReadCap;
pub use read::{ReadError, read_capped, read_text_capped};
pub use update::{UpdateError, update_atomic};
pub use write::{NewFileMode, WriteError, write_atomic};
