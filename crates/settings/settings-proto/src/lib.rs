//! The wire vocabulary of live settings modules (`org.bardo.SettingsModule1`), without a bus.
//!
//! A daemon that serves runtime state and the Settings app that reads it agree on these types;
//! `settings-dbus` carries them over the session bus. A program that only builds or checks a
//! schema, or only needs the interface text, depends on this crate and links no D-Bus library.
//!
//! - [`LiveSchema`] is what `Describe` returns: keys in the same shape as a file schema.
//! - [`Snapshot`] is what `Snapshot` returns: the schema and every key's value, agreeing.
//! - [`limits`] is how much a module may send.
//! - [`Timeouts`] is how long a reader waits on a module.
//! - [`INTERFACE`] and [`INTROSPECTION_XML`] name the frozen D-Bus interface.

pub mod limits;
mod schema;
mod snapshot;
mod timeouts;

pub use schema::{LiveSchema, LiveSchemaError};
pub use snapshot::Snapshot;
pub use timeouts::Timeouts;

/// The D-Bus interface name.
pub const INTERFACE: &str = "org.bardo.SettingsModule1";

/// The introspection XML of [`INTERFACE`], the frozen wire form.
pub const INTROSPECTION_XML: &str = include_str!("org.bardo.SettingsModule1.xml");
