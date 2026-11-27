//! The allow-list network mode's filtering proxy, std only. The host names, patterns and the
//! policy that gives the verdicts are pure data in `sandbox_core::egress`; this is what serves them.
//!
//! - `request`: the one request line the proxy understands, read with bounded sizes.
//! - `resolve`: name resolution behind a trait.
//! - `proxy`: the [`Proxy`] that listens on the unix socket and copies bytes.

mod proxy;
mod request;
mod resolve;

pub use proxy::{Entry, Gate, Journal, Outcome, Proxy, Silent};
pub use request::{BadRequestError, Head, MAX_LINE, Request, Target, parse_head, read_head};
pub use resolve::{Resolver, SystemResolver};
