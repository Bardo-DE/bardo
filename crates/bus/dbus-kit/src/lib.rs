//! What every D-Bus service and client of the desktop shares.
//!
//! - [`caller`]: `SenderName` and `Verdict`, the words for who called and whether they may.
//! - [`connect`]: which bus to connect to, and the connection.
//! - [`name`]: taking a well-known name without queueing for it.
//! - `ready` (feature `tokio`): waiting until a connection's object server takes calls, before the name is taken.
//! - [`Call`], [`Unserved`] and [`Document`]: what `#[bus_interface]` (from `dbus-kit-macros`)
//!   expands to, so an interface written once as the client's proxy trait also yields the server
//!   trait and the object that serves it.
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

// The macro's expansion names `::dbus_kit::Call`; this makes that path work in this crate too.
extern crate self as dbus_kit;

pub mod caller;
pub mod connect;
pub mod name;
#[cfg(feature = "tokio")]
pub mod ready;

mod serve;

pub use dbus_kit_macros::bus_interface;
pub use serve::{Call, Document, Unserved};
