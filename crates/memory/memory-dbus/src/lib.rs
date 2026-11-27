//! The memory service's D-Bus API, the caller's half (memory.md section 3.10): `org.bardo.Memory1` at
//! `/org/bardo/Memory1` as three interfaces (`Record`, `Recall`, `Control`) with a zbus proxy
//! each, the argument codec between `memory-core` requests and the member arguments, `invoke`
//! (a call over a connection) and the typed errors. Bodies are the serde JSON of `memory-core`
//! types in `s` arguments. The daemon's half (skeletons, served objects, signals, the
//! introspection checked against `dbus/org.bardo.Memory1.xml`) is the `serve`, `skeleton` and
//! `introspect` modules.

mod codec;
mod control;
mod error;
mod introspect;
mod invoke;
mod memory_recall;
mod names;
mod record;
pub mod roles;
mod serve;
mod skeleton;

pub use codec::{
    Call, CallArg, CodecError, Iface, decode_reply, decode_request, encode_reply, encode_request,
};
pub use control::ControlProxy;
pub use error::{Failure, MemoryError};
pub use introspect::{INTROSPECTION_FILE, introspection};
pub use invoke::invoke;
pub use memory_recall::RecallProxy;
pub use names::{
    CONTROL_INTERFACE, ERROR_PREFIX, MEMORY_BUS, MEMORY_PATH, RECALL_INTERFACE, RECORD_INTERFACE,
};
pub use record::RecordProxy;
pub use serve::{
    CONFINEMENT_OFF, ControlObject, RecallObject, RecordObject, Serve, Signal, emit, emit_to,
    serve_on, served_introspection,
};
/// The session-bus connection transports and daemons hold.
pub use zbus::Connection as BusConnection;
