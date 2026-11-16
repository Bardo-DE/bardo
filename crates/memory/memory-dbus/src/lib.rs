//! almanac's D-Bus API, the caller's half (memory.md section 3.10): `org.bardo.Memory1` at
//! `/org/bardo/Memory1` as three interfaces (`Record`, `Recall`, `Control`) with a zbus proxy
//! each, the argument codec between `memory-core` requests and the member arguments, `invoke`
//! (a call over a connection) and the typed errors. Bodies are the serde JSON of `memory-core`
//! types in `s` arguments. The daemon's half (skeletons, served objects, signals, the
//! introspection checked against `dbus/org.bardo.Memory1.xml`) is `memory-dbus-serve`.

mod codec;
mod control;
mod error;
mod invoke;
mod memory_recall;
mod names;
mod record;

pub use codec::{
    Call, CallArg, CodecError, Iface, decode_reply, decode_request, encode_reply, encode_request,
};
pub use control::ControlProxy;
pub use error::{Failure, MemoryError};
pub use invoke::invoke;
pub use memory_recall::RecallProxy;
pub use names::{
    CONTROL_INTERFACE, ERROR_PREFIX, MEMORY_BUS, MEMORY_PATH, RECALL_INTERFACE, RECORD_INTERFACE,
};
pub use record::RecordProxy;
/// The session-bus connection transports and daemons hold.
pub use zbus::Connection as BusConnection;
