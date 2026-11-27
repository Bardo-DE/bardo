//! What Tailscale's `LocalAPI` says: this computer's Tailscale of itself, its network and the
//! computers on it, who an address on the network belongs to, and the notices of its watch
//! stream, as typed values with the readers of the answers. Nothing here links Tailscale's code or
//! reads what its program prints, and nothing reaches the socket: the client is `tailscale-io`.
//!
//! Field names are those of Tailscale v1.80.0 (BSD-3-Clause); each reader says which file it
//! follows.

mod error;
mod machine;
mod notice;
mod object;
mod status;
mod time;
mod whois;

pub use error::{Cause, TailscaleError};
pub use machine::{Machine, MachineOwner, NodeId};
pub use notice::Notice;
pub use status::{Backend, Node, Standing, Status, Tailnet, User, UserId};
pub use whois::{WhoIs, WhoIsNode};
