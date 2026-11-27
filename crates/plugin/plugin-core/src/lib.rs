//! The pure half of running plugins as separate programs: what a plugin and a host agree on,
//! with no process, no file and no clock.
//!
//! A plugin is an executable a person installs. The host never links it: it describes itself in a
//! TOML manifest, the host finds the manifest, starts the program for a request, and talks to it
//! over a pipe in length-prefixed JSON frames. This crate holds everything about that which
//! needs no operating system; `plugin-host` finds the manifests on disk and runs the programs.
//!
//! The crate is generic over the host's *capability type*, the things a plugin can offer. It owns
//! the mechanics and none of the vocabulary:
//!
//! - [`wire`]: frames, the blocking reader and writer a plugin uses, and the incremental
//!   decoder a host uses. A plugin program depends on this crate for this alone.
//! - [`manifest`]: the envelope every manifest has (`id`, `name`, `protocol`, optional
//!   `version`, `licence` and `summary`, `[program]`), with each `[[provides]]` table handed to
//!   the host's own [`manifest::Provides`] type, or kept raw for an owner to read later.
//! - [`registry`]: which plugin serves a request, independent of the order the disk listed files
//!   in.
//! - [`reparse_with`]: the second step of a two-step read, raw entries to an owner's own type.
//! - [`suggest`]: the package to name when none does.
//! - [`Timeouts`]: how long a host waits on a plugin.

mod capability;
#[cfg(test)]
mod fixture;
pub mod manifest;
pub mod registry;
mod reparse;
mod suggest;
mod timeouts;
pub mod wire;

pub use capability::Capability;
pub use reparse::{Reparsed, Skipped, reparse_with};
pub use suggest::{Package, Suggestion, suggest};
pub use timeouts::Timeouts;
