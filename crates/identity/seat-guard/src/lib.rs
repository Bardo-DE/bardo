//! The guards of a socket served to every local account: who may connect, who is the shell, whose
//! turn it is to complain and how long a listener pauses.
//!
//! Policy-free mechanics: the table of callers, the caps and the messages stay with the daemon that
//! serves the socket. The shell check asks the caller-identity library for a standing and holds it
//! to the daemon's [`caller_identity_core::Requirement`]s, like every other role check.

mod accept;
mod seat;
mod shell;

pub use accept::accept_failure;
pub use seat::{SEAT_FILE, SeatFile, active_uid};
pub use shell::{Held, Origin, SeatUser, ShellCheck, Vouch};
