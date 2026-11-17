//! What every D-Bus service and client of the desktop shares.
//!
//! - [`connect`]: which bus to connect to, and the connection.
//! - [`name`]: taking a well-known name without queueing for it.
//! - `ready` (feature `tokio`): waiting until a connection's object server takes calls, before the name is taken.
//! - [`Call`], [`Unserved`] and [`Document`]: what `#[bus_interface]` (from `dbus-kit-macros`)
//!   expands to, so an interface written once as the client's proxy trait also yields the server
//!   trait and the object that serves it.

// The macro's expansion names `::dbus_kit::Call`; this makes that path work in this crate too.
extern crate self as dbus_kit;

pub mod connect;
pub mod name;
#[cfg(feature = "tokio")]
pub mod ready;

mod serve;

pub use dbus_kit_macros::bus_interface;
pub use serve::{Call, Document, Unserved};
