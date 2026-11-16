//! The files memory keeps beside its Spaces, for whoever hosts the service: memoryd, or an app
//! that hosts it in process (`memory-local`).
//!
//! - [`Locator`] finds `$XDG_CONFIG_HOME/memory/settings.toml` (then each of `$XDG_CONFIG_DIRS`),
//!   reads it through `memory-settings`' lenient reader and gate, and keeps the person's
//!   confirmations of loosened values in a store of its own.
//! - [`read_config`] reads a config file whole, with a size cap.
//!
//! The settings model and the service read no file; they are handed what this crate reads.

mod files;
mod locate;

pub use files::{CONFIG_CAP, read_config};
pub use locate::Locator;
