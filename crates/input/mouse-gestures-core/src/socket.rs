//! The socket's decisions (12.3.10): whom to serve, what each subscriber is, what a reader's
//! message does and when the reader is told. The daemon's socket thread gathers the facts that
//! need a syscall, feeds them in as [`SocketEvent`]s and carries out the [`SocketEffect`]s that
//! come back.
//!
//! The socket turns subscribers' `PointerOver` reports into the one claim the tracker needs
//! (11.3.14: while any subscriber has the pointer, foreign outputs are silent), and their
//! `SessionLock` reports into whether the session is locked (a locked session runs no gesture
//! action). The mouse gestures daemon is a system service, so its socket is open to every local
//! account and only the person at the seat is served (`seat`); of their programs only the shell is
//! believed about the pointer and an unlocked session.
//!
//! `model` holds the subscribers and the events and effects, `step` (with `admit`, `packet`,
//! `deliver` and `publish`) is the one transition `(Subscribers, SocketEvent, now) -> (Subscribers,
//! effects)`. No descriptor lives here: a subscriber is an id, and its uid, what it claims and who
//! has the seat arrive inside events.

mod admit;
mod aggregate;
mod deliver;
mod limits;
mod model;
mod packet;
mod publish;
mod seat;
mod step;
#[cfg(test)]
mod tests;

pub use limits::MAX_CLIENTS;
pub use model::{ClientId, SocketEffect, SocketEvent, Subscribers};

use mouse_gestures_proto::{DeviceInfo, HostAction, ToHost};

/// What the reader thread tells the socket thread.
#[derive(Debug, Clone, PartialEq)]
pub enum ToSocket {
    /// A mouse was opened: remembered for every later `Hello`, and announced now.
    Device(DeviceInfo),
    /// The mouse went away (after the tracker's `DeviceGone`).
    Gone,
    /// A message for every admitting subscriber.
    Msg(ToHost),
    /// An action for the shell of the person at the seat, and for nobody else.
    Run(HostAction),
}

/// What the socket thread tells the reader: the subscribers' joint state, when it changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FromSocket {
    /// The pointer is over a subscriber's surface, or not (11.3.14).
    Pointer(bool),
    /// A subscriber says the session is locked, or none does any more.
    Session(bool),
    /// The shell sent the person's settings (`None`: it sent none, or the person left the seat).
    UserConfig(Option<String>),
}
