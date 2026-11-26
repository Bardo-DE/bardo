//! The model tap: test and acceptance support, not shipped. It watches a daemon's inferd link
//! (through `action-dbus`'s `InferWatch`) and keeps each chat request and its answer as a
//! `ModelExchange`, in memory (`MemoryTap`) or appended to a file (`Tap::trace_file`).
//!
//! It writes prompts to disk, which is why it is its own crate: no shipped crate lists it
//! (`cargo xtask check layers` checks every manifest), so a release daemon has no code that
//! could do so.

mod exchange;
mod tap;

pub use tap::{MemoryTap, Tap};
