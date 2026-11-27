//! What the egress proxy lets through: host names and patterns, and the policy that gives the
//! verdicts. Pure, so a table tests it; the proxy that serves them is in the `sandbox` crate.

mod host;
mod policy;

pub use host::{EgressHostError, Host, HostPattern};
pub use policy::{DEFAULT_PORTS, EgressPolicy, EgressRefusedError, PolicyError, Ports, special};
