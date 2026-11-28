//! The input broker wire: what a client may ask for and what the service answers, as values and as
//! datagrams, plus the fixed list of device kinds.
//!
//! This crate is pure. It reads no file, clock or socket and links nothing but `thiserror` and the pure caller-identity vocabulary (the role names of the two daemons it names); the
//! daemon and the client both build on it. A message is one `SOCK_SEQPACKET` datagram: [`codec`]
//! turns it into a [`message::Request`] or a [`message::Reply`] and back. What the service does
//! with a request (the per-event ceilings, the rate limit, the connection caps) is policy and
//! lives in `input-broker-core`.
#![forbid(unsafe_code)]

pub mod codec;
pub mod codes;
pub mod event;
pub mod kind;
pub mod limits;
pub mod maker;
pub mod message;
pub mod spec;

pub use codec::{WireError, decode_reply, decode_request, encode_reply, encode_request};
pub use event::Event;
pub use kind::{Delivery, Kind};
pub use maker::{KEYMAP_DAEMON, Maker, PAD_DAEMON};
pub use message::{Create, DeviceId, PointerCaps, Reason, Reply, Request};
pub use spec::{AbsAxis, DeviceSpec};
