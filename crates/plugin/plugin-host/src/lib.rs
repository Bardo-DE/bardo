//! Run plugins as separate programs: find their manifests on disk and start them.
//!
//! A plugin is an executable a person installs. The host never links it: it describes itself in a
//! TOML manifest, the host finds the manifest, starts the program for a request, and talks to it
//! over a pipe in length-prefixed JSON frames. A plugin that crashes, hangs or lies costs one
//! request and never takes the host down.
//!
//! This crate is the half that touches the machine. What a plugin and a host agree on (the wire,
//! the manifest, the registry) is `plugin-core`, which a plugin program depends on alone.
//!
//! - [`discover`], [`scan`] and [`reparse`]: finding manifests in `<data dir>/<app>/plugins` or in
//!   a folder for each plugin, with the app name a parameter, and checking the programs they
//!   name.
//! - [`run`] (Unix): spawning, greeting, timeouts, cancellation, crash containment.

mod discover;
#[cfg(test)]
mod fixture;
#[cfg(unix)]
pub mod run;

pub use discover::{Discovery, Layout, Rejected, Scan, Search, discover, reparse, scan};
