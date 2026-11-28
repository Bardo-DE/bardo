//! The input broker's decisions, without its sockets, clocks or kernel: who may have which device,
//! the per-event ceilings, the rate limit, the connection caps and one connection's state machine.
//!
//! Everything here is a value in and a value out. Time is an argument, the kernel and the table of
//! callers are seams ([`backend::Backend`], [`identity::SeatSource`]) and nothing reads a file,
//! starts a thread or opens a device, so the whole policy is tested with plain values. The datagrams
//! themselves are `input-broker-wire`; the root service that holds the sockets and drives these is
//! `input-broker-daemon`.
//!
//! - [`ceiling`]: for each kind of device, what the service makes and what every event may be.
//! - [`policy`]: which caller may have which kind.
//! - [`session`]: one connection, requests in and replies out.
//! - [`slots`] and [`pending`]: who may hold a connection, and who may wait to be identified.
//! - [`limiter`], [`idle`]: the event rate and the quiet timer.
#![forbid(unsafe_code)]

pub mod backend;
pub mod ceiling;
pub mod identity;
pub mod idle;
pub mod keytable;
pub mod limiter;
pub mod limits;
pub mod pending;
pub mod policy;
pub mod session;
pub mod slots;
mod tally;
